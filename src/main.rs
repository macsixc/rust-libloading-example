use std::ffi::OsStr;

use libc::c_int;
use libloading::os::unix::Symbol as RawSymbol;
use libloading::{Library, Symbol};

#[repr(C)]
pub struct Object {
    _private: [u8; 0],
}
type FreeObject = extern "C" fn(*mut Object);
type Init = extern "C" fn() -> *mut Object;
type GetApiVersion = extern "C" fn() -> c_int;
type GetInfo = extern "C" fn(*const Object) -> c_int;
type SetInfo = extern "C" fn(*mut Object, c_int);

pub trait VTable {
    unsafe fn new(library: &Library) -> Self;
    unsafe fn free(&self, object: *mut Object);
}

struct VTableV0 {
    free_object: RawSymbol<FreeObject>,
    get_info: RawSymbol<GetInfo>,
    set_info: RawSymbol<SetInfo>,
}

impl VTable for VTableV0 {
    unsafe fn new(library: &Library) -> Self {
        println!("Loading API version 0...");
        let free_object: Symbol<FreeObject> = library.get(b"free_object\0").unwrap();
        let free_object = free_object.into_raw();
        let get_info: Symbol<GetInfo> = library.get(b"get_info\0").unwrap();
        let get_info = get_info.into_raw();
        let set_info: Symbol<SetInfo> = library.get(b"set_info\0").unwrap();
        let set_info = set_info.into_raw();

        VTableV0 {
            free_object,
            get_info,
            set_info,
        }
    }
    unsafe fn free(&self, object: *mut Object)
    {
        (&self.free_object)(object);
    }
}

struct Plugin<T: VTable> {
    #[allow(dead_code)]
    library: Library,
    object: *mut Object,
    vtable: T,
}

impl<T: VTable> Plugin<T>{
    unsafe fn new(library_name: &OsStr) -> Plugin<T> {
        let library = Library::new(library_name).unwrap();
        let get_api_version: Symbol<GetApiVersion> = library.get(b"get_api_version\0").unwrap();
        let vtable = match get_api_version() {
            0 => T::new(&library),
            _ => panic!("Unrecognized C API version number."),
        };

        let init: Symbol<Init> = library.get(b"init\0").unwrap();
        let object: *mut Object = init();

        Plugin {
            library,
            object,
            vtable,
        }
    }
}

impl<T: VTable> Drop for Plugin<T> {
    fn drop(&mut self) {
        unsafe
        {
            self.vtable.free(self.object);
        }
    }
}

fn main() {
    let library_path: &OsStr = OsStr::new("ffi-test/libffi-test.so");
    let plugin = unsafe { Plugin::<VTableV0>::new(library_path) };

    println!(
        "Original value: {}",
        (plugin.vtable.get_info)(plugin.object)
    );
    (plugin.vtable.set_info)(plugin.object, 42);

    println!("New value: {}", (plugin.vtable.get_info)(plugin.object));
}
