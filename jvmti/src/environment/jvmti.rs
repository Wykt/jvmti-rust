
use crate::native::jvmti_native::{jclass, Struct__jvmtiClassDefinition};

use super::super::capabilities::Capabilities;
use super::super::class::{ClassId, ClassSignature, JavaType};
use super::super::error::{wrap_error, NativeError};
use super::super::event::{EventCallbacks, VMEvent};
use super::super::event_handler::*;
use super::super::mem::MemoryAllocation;
use super::super::method::{MethodId, MethodSignature};
use super::super::thread::{ThreadId, Thread};
use super::super::util::stringify;
use super::super::version::VersionNumber;
use super::super::native::{MutString, MutByteArray, JavaClass, JavaObject, JavaInstance, JavaLong, JavaThread, JVMTIEnvPtr};
use super::super::native::jvmti_native::{Struct__jvmtiThreadInfo, jvmtiCapabilities};
use std::ptr::{self, null_mut};
use libc::c_int;

pub trait JVMTI {

    ///
    /// Return the JVM TI version number, which includes major, minor and micro version numbers.
    ///
    fn get_version_number(&self) -> VersionNumber;
    /// Set new capabilities by adding the capabilities whose values are set to true in new_caps.
    /// All previous capabilities are retained.
    /// Some virtual machines may allow a limited set of capabilities to be added in the live phase.
    fn add_capabilities(&mut self, new_capabilities: &Capabilities) -> Result<Capabilities, NativeError>;
    fn get_capabilities(&self) -> Capabilities;
    fn get_class_loader_loaded_classes(&self, class_loader: JavaObject) -> Result<Vec<JavaClass>, NativeError>;
    fn get_loaded_classes(&self) -> Result<Vec<JavaClass>, NativeError>;
    /// Set the functions to be called for each event. The callbacks are specified by supplying a
    /// replacement function table. The function table is copied--changes to the local copy of the
    /// table have no effect. This is an atomic action, all callbacks are set at once. No events
    /// are sent before this function is called. When an entry is None no event is sent.
    /// An event must be enabled and have a callback in order to be sent--the order in which this
    /// function and set_event_notification_mode are called does not affect the result.
    fn set_event_callbacks(&mut self, callbacks: EventCallbacks) -> Option<NativeError>;
    fn set_event_notification_mode(&mut self, event: VMEvent, mode: bool) -> Option<NativeError>;
    fn redefine_classes(&self, class_definitions: &[JVMTIClassDefinition]) -> Result<(), NativeError>;
    fn retransform_classes(&self, classes: &[JavaClass]) -> Result<(), NativeError>;
    fn get_all_threads(&self) -> Result<Vec<JavaThread>, NativeError>;
    fn get_thread_info(&self, thread_id: &JavaThread) -> Result<Thread, NativeError>;
    fn get_method_declaring_class(&self, method_id: &MethodId) -> Result<ClassId, NativeError>;
    fn get_method_name(&self, method_id: &MethodId) -> Result<MethodSignature, NativeError>;
    fn get_class_signature(&self, class_id: &ClassId) -> Result<ClassSignature, NativeError>;
    fn allocate(&self, len: usize) -> Result<MemoryAllocation, NativeError>;
    fn deallocate(&self);
}

pub struct JVMTIEnvironment {

    jvmti: JVMTIEnvPtr
}

pub struct JVMTIClassDefinition {
    pub class: JavaClass,
    pub class_data: Vec<u8>
}

impl JVMTIEnvironment {
    pub fn new(env_ptr: JVMTIEnvPtr) -> JVMTIEnvironment {
        JVMTIEnvironment { jvmti: env_ptr }
    }
}

impl JVMTI for JVMTIEnvironment {

    fn get_version_number(&self) -> VersionNumber {
        unsafe {
            let mut version: i32 = 0;
            let version_ptr = &mut version;
            (**self.jvmti).GetVersionNumber.unwrap()(self.jvmti, version_ptr);
            let uversion = *version_ptr as u32;
            VersionNumber::from_u32(&uversion)
        }
    }

    fn get_loaded_classes(&self) -> Result<Vec<JavaClass>, NativeError> {
        let mut classes_count_ptr: c_int = 0;
        let mut classes_ptr: *mut jclass = null_mut();

        unsafe {
            match wrap_error((**self.jvmti).GetLoadedClasses.unwrap()(self.jvmti, &mut classes_count_ptr, &mut classes_ptr)) {
                NativeError::NoError => {
                    let mut classes = Vec::<jclass>::new();

                    for _ in 0..classes_count_ptr {
                        let class = JavaClass::from(classes_ptr.read());
                        classes_ptr = classes_ptr.add(1);

                        classes.push(class)
                    }

                    Ok(classes)
                }
                err  => Err(err)
            } 
        }
    }

    fn get_class_loader_loaded_classes(&self, class_loader: JavaObject) -> Result<Vec<JavaClass>, NativeError> {
        let mut classes_count_ptr: c_int = 0;
        let mut classes_ptr: *mut jclass = null_mut();

        unsafe {
            match wrap_error((**self.jvmti).GetClassLoaderClasses.unwrap()(self.jvmti, class_loader, &mut classes_count_ptr, &mut classes_ptr)) {
                NativeError::NoError => {
                    let mut classes = Vec::<jclass>::new();

                    for _ in 0..classes_count_ptr {
                        let class = JavaClass::from(classes_ptr.read());
                        classes_ptr = classes_ptr.add(1);

                        classes.push(class)
                    }

                    Ok(classes)
                }
                err  => Err(err)
            } 
        }
    }

    fn retransform_classes(&self, classes: &[JavaClass]) -> Result<(), NativeError> {
        let class_count: i32 = classes.len() as i32;
        let classes = classes.as_ptr();

        unsafe {
            match wrap_error((**self.jvmti).RetransformClasses.unwrap()(self.jvmti, class_count, classes)) {
                NativeError::NoError => Ok(()),
                err  => Err(err)
            }
        }
    }

    fn redefine_classes(&self, class_definitions: &[JVMTIClassDefinition]) -> Result<(), NativeError> {
        let classes_count: c_int = class_definitions.len() as i32;
        let mut jvmti_class_definitions = Vec::<Struct__jvmtiClassDefinition>::new();

        for class_def in class_definitions {
            jvmti_class_definitions.push(Struct__jvmtiClassDefinition {
                klass: class_def.class,
                class_byte_count: class_def.class_data.len() as i32,
                class_bytes: class_def.class_data.as_ptr(),
            })
        }

        unsafe {
            match wrap_error((**self.jvmti).RedefineClasses.unwrap()(self.jvmti, classes_count, jvmti_class_definitions.as_ptr())) {
                NativeError::NoError => {
                    Ok(())
                },
                err  => Err(err)
            }
        }
    }

    fn get_all_threads(&self) -> Result<Vec<JavaThread>, NativeError> {
        let mut threads_count_ptr: c_int = 0;
        let mut threads_ptr: *mut JavaThread = ptr::null_mut();

        unsafe {
            match wrap_error((**self.jvmti).GetAllThreads.unwrap()(self.jvmti, &mut threads_count_ptr, &mut threads_ptr)) {
                NativeError::NoError => {
                    let mut vec = Vec::new();

                    for _ in 0..threads_count_ptr {
                        let thread = threads_ptr.read();
                        threads_ptr = threads_ptr.add(1);

                        vec.push(JavaThread::from(thread))
                    }

                    Ok(vec)
                }
                err  => Err(err)
            }
        }
    }

    fn add_capabilities(&mut self, new_capabilities: &Capabilities) -> Result<Capabilities, NativeError> {
        let native_caps = new_capabilities.to_native();
        let caps_ptr:*const jvmtiCapabilities = &native_caps;

        unsafe {
            match wrap_error((**self.jvmti).AddCapabilities.unwrap()(self.jvmti, caps_ptr)) {
                NativeError::NoError => Ok(self.get_capabilities()),
                err  => Err(err)
            }
        }
    }

    fn get_capabilities(&self) -> Capabilities {
        unsafe {
            let caps = Capabilities::new();
            let mut native_caps = caps.to_native();
            {
                let cap_ptr = &mut native_caps;
                (**self.jvmti).GetCapabilities.unwrap()(self.jvmti, cap_ptr);
            }
            Capabilities::from_native(&native_caps)
        }
    }

    fn set_event_callbacks(&mut self, callbacks: EventCallbacks) -> Option<NativeError> {
        register_vm_init_callback(callbacks.vm_init);
        register_vm_start_callback(callbacks.vm_start);
        register_vm_death_callback(callbacks.vm_death);
        register_vm_object_alloc_callback(callbacks.vm_object_alloc);
        register_method_entry_callback(callbacks.method_entry);
        register_method_exit_callback(callbacks.method_exit);
        register_thread_start_callback(callbacks.thread_start);
        register_thread_end_callback(callbacks.thread_end);
        register_exception_callback(callbacks.exception);
        register_exception_catch_callback(callbacks.exception_catch);
        register_monitor_wait_callback(callbacks.monitor_wait);
        register_monitor_waited_callback(callbacks.monitor_waited);
        register_monitor_contended_enter_callback(callbacks.monitor_contended_enter);
        register_monitor_contended_endered_callback(callbacks.monitor_contended_entered);
        register_field_access_callback(callbacks.field_access);
        register_field_modification_callback(callbacks.field_modification);
        register_garbage_collection_start(callbacks.garbage_collection_start);
        register_garbage_collection_finish(callbacks.garbage_collection_finish);
        register_class_file_load_hook(callbacks.class_file_load_hook);

        let (native_callbacks, callbacks_size) = registered_callbacks();

        unsafe {
            match wrap_error((**self.jvmti).SetEventCallbacks.unwrap()(self.jvmti, &native_callbacks, callbacks_size)) {
                NativeError::NoError => None,
                err  => Some(err)
            }
        }
    }

    fn set_event_notification_mode(&mut self, event: VMEvent, mode: bool) -> Option<NativeError> {
        unsafe {
            let mode_i = match mode { true => 1, false => 0 };
            let sptr: JavaObject = ptr::null_mut();

            match wrap_error((**self.jvmti).SetEventNotificationMode.unwrap()(self.jvmti, mode_i, event as u32, sptr)) {
                NativeError::NoError => None,
                err  => Some(err)
            }
        }
    }

    fn get_thread_info(&self, thread_id: &JavaThread) -> Result<Thread, NativeError> {
        let mut info = Struct__jvmtiThreadInfo { name: ptr::null_mut(), priority: 0, is_daemon: 0, thread_group: ptr::null_mut(), context_class_loader: ptr::null_mut()};
        let info_ptr = &mut info;

        unsafe {
            match (**self.jvmti).GetThreadInfo {
                Some(func) => {
                    match wrap_error(func(self.jvmti, *thread_id, info_ptr)) {
                        NativeError::NoError => Ok(Thread {
                            id: ThreadId { native_id: *thread_id },
                            name: stringify(info_ptr.name),
                            priority: info_ptr.priority as u32,
                            is_daemon: info_ptr.is_daemon > 0,
                            context_class_loader: &mut *info_ptr.context_class_loader
                        }),
                        err => Err(err)
                    }
                },
                None => Err(NativeError::NoError)
            }
        }
    }

    fn get_method_declaring_class(&self, method_id: &MethodId) -> Result<ClassId, NativeError> {
        let mut jstruct: JavaInstance = JavaInstance { _hacky_hack_workaround: 0 };
        let mut jclass_instance: JavaClass = &mut jstruct;
        let meta_ptr: *mut JavaClass = &mut jclass_instance;

        unsafe {
            match wrap_error((**self.jvmti).GetMethodDeclaringClass.unwrap()(self.jvmti, method_id.native_id, meta_ptr)) {
                NativeError::NoError => Ok(ClassId { native_id: *meta_ptr }),
                err => Err(err)
            }
        }
    }

    fn get_method_name(&self, method_id: &MethodId) -> Result<MethodSignature, NativeError> {
        let mut method_name = ptr::null_mut();
        let method_ptr = &mut method_name;

        let mut signature: MutString = ptr::null_mut();
        let signature_ptr = &mut signature;

        let mut generic_sig: MutString = ptr::null_mut();
        let generic_sig_ptr = &mut generic_sig;

        unsafe {
            match wrap_error((**self.jvmti).GetMethodName.unwrap()(self.jvmti, method_id.native_id, method_ptr, signature_ptr, generic_sig_ptr)) {
                NativeError::NoError => Ok(MethodSignature::new(stringify(*method_ptr))),
                err => Err(err)
            }
        }
    }

    fn get_class_signature(&self, class_id: &ClassId) -> Result<ClassSignature, NativeError> {
        unsafe {
            let mut native_sig: MutString = ptr::null_mut();
            let mut sig: MutString = ptr::null_mut();
            let p1: *mut MutString = &mut sig;
            let p2: *mut MutString = &mut native_sig;

            match wrap_error((**self.jvmti).GetClassSignature.unwrap()(self.jvmti, class_id.native_id, p1, p2)) {
                NativeError::NoError => Ok(ClassSignature::new(&JavaType::parse(&stringify(sig)).unwrap())),
                err => Err(err)
            }
        }
    }

    fn allocate(&self, len: usize) -> Result<MemoryAllocation, NativeError> {
        let size: JavaLong = len as JavaLong;
        let mut ptr: MutByteArray = ptr::null_mut();
        let mem_ptr: *mut MutByteArray = &mut ptr;

        unsafe {
            match wrap_error((**self.jvmti).Allocate.unwrap()(self.jvmti, size, mem_ptr)) {
                NativeError::NoError => Ok(MemoryAllocation { ptr, len }),
                err => Err(err)
            }
        }
    }

    fn deallocate(&self) {

    }
}
