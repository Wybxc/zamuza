//! Tiny C Compiler bindings for Rust
//!
//! # Example
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use std::ffi::CString;
//!
//! let src = CString::new(r#"
//!     #include <stdio.h>
//!     int main() {
//!         printf("Hello, world!\n");
//!         return 0;
//!     }
//! "#)?;
//!
//! let ret = tinycc::Context::new(tinycc::OutputType::Memory)?
//!     .compile_string(&src)?
//!     .run(&[])?;
//!
//! assert_eq!(ret, 0);
//! # Ok(())
//! # }
//! ```

use std::{
    ffi::{c_char, c_int, c_void, CStr, CString},
    marker::PhantomData,
    path::Path,
    ptr::NonNull,
    sync::atomic::{AtomicBool, Ordering},
};
use thiserror::Error;

mod bindings;

/// tcc error
#[derive(Debug, Error)]
pub enum Error {
    #[error("tcc context is already initialized")]
    AlreadyInitialized,

    #[error("initizalization failed")]
    InitializationFailed,

    #[error("library not found")]
    LibraryNotFound,

    #[error("relocation failed")]
    RelocationFailed,

    #[error("tcc error: {}", .msgs.join("\n"))]
    TCCError { msgs: Vec<String> },
}

/// output type
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum OutputType {
    /// output will be run in memory
    Memory = bindings::TCC_OUTPUT_MEMORY,
    /// executable file
    Exe = bindings::TCC_OUTPUT_EXE,
    /// dynamic library
    Dll = bindings::TCC_OUTPUT_DLL,
    /// object file
    Obj = bindings::TCC_OUTPUT_OBJ,
    /// only preprocess
    Preprocess = bindings::TCC_OUTPUT_PREPROCESS,
}

static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// tcc compilation context
pub struct Context {
    inner: *mut bindings::TCCState,
    errors: Box<Option<Vec<String>>>,
    _marker: PhantomData<bindings::TCCState>,
}

impl Context {
    /// create a new TCC compilation context
    pub fn new(output_type: OutputType) -> Result<Self, Error> {
        if INITIALIZED.swap(true, Ordering::Acquire) {
            return Err(Error::AlreadyInitialized);
        }

        let inner = unsafe { bindings::tcc_new() };

        if inner.is_null() {
            return Err(Error::InitializationFailed);
        }

        let mut context = Self {
            inner,
            errors: Box::new(None),
            _marker: PhantomData,
        };

        extern "C" fn err_callback(opaque: *mut c_void, msg: *const c_char) {
            let errors = unsafe { &mut *(opaque as *mut Option<Vec<String>>) };
            let msg = unsafe { CStr::from_ptr(msg) };
            if errors.is_none() {
                *errors = Some(Vec::new());
            }
            errors
                .as_mut()
                .unwrap()
                .push(msg.to_string_lossy().into_owned());
        }

        unsafe {
            bindings::tcc_set_error_func(
                context.inner,
                context.errors.as_mut() as *mut Option<Vec<String>> as *mut _,
                Some(err_callback),
            )
        }

        unsafe { bindings::tcc_set_output_type(context.inner, output_type as c_int) };

        Ok(context)
    }

    /// set CONFIG_TCCDIR at runtime
    pub fn set_lib_path<T: AsRef<Path>>(self, path: T) -> Self {
        let path = to_cstr(path);
        unsafe { bindings::tcc_set_lib_path(self.inner, path.as_ptr()) };
        self
    }

    /// set options as from command line (multiple supported)
    pub fn set_options(mut self, options: &CStr) -> Result<Self, Error> {
        *self.errors = None;
        let ret = unsafe { bindings::tcc_set_options(self.inner, options.as_ptr()) };
        if ret >= 0 {
            Ok(self)
        } else {
            let msgs = self.errors.take().unwrap();
            Err(Error::TCCError { msgs })
        }
    }

    /// add include path
    pub fn add_include_path<T: AsRef<Path>>(self, path: T) -> Self {
        let path = to_cstr(path);
        let ret = unsafe { bindings::tcc_add_include_path(self.inner, path.as_ptr()) };
        assert_eq!(ret, 0);
        self
    }

    /// add in system include path
    pub fn add_sysinclude_path<T: AsRef<Path>>(self, path: T) -> Self {
        let path = to_cstr(path);
        let ret = unsafe { bindings::tcc_add_sysinclude_path(self.inner, path.as_ptr()) };
        assert_eq!(ret, 0);
        self
    }

    /// define preprocessor symbol 'sym'. value can be NULL, sym can be "sym=val"
    pub fn define_symbol(self, sym: &CStr, val: Option<&CStr>) -> Self {
        unsafe {
            bindings::tcc_define_symbol(
                self.inner,
                sym.as_ptr(),
                val.map(|v| v.as_ptr()).unwrap_or(std::ptr::null()),
            )
        };
        self
    }

    /// undefine preprocess symbol 'sym'
    pub fn undefine_symbol(self, sym: &CStr) -> Self {
        unsafe { bindings::tcc_undefine_symbol(self.inner, sym.as_ptr()) };
        self
    }

    /// equivalent to -Lpath option
    pub fn add_library_path<T: AsRef<Path>>(self, path: T) -> Self {
        let path = to_cstr(path);
        let ret = unsafe { bindings::tcc_add_library_path(self.inner, path.as_ptr()) };
        assert_eq!(ret, 0);
        self
    }

    /// the library name is the same as the argument of the '-l' option
    pub fn add_library(self, libname: &CStr) -> Result<Self, Error> {
        let ret = unsafe { bindings::tcc_add_library(self.inner, libname.as_ptr()) };
        if ret >= 0 {
            Ok(self)
        } else {
            Err(Error::LibraryNotFound)
        }
    }

    /// add a symbol to the compiled program
    pub fn add_symbol(self, name: &CStr, val: Symbol) -> Self {
        let ret = unsafe { bindings::tcc_add_symbol(self.inner, name.as_ptr(), val.as_ptr()) };
        assert_eq!(ret, 0);
        self
    }

    /// add a file (C file, dll, object, library, ld script).
    pub fn compile_file<T: AsRef<Path>>(mut self, filename: T) -> Result<Self, Error> {
        *self.errors = None;
        let filename = to_cstr(filename);
        let ret = unsafe { bindings::tcc_add_file(self.inner, filename.as_ptr()) };
        if ret >= 0 {
            Ok(self)
        } else {
            let msgs = self.errors.take().unwrap();
            Err(Error::TCCError { msgs })
        }
    }

    /// compile a string containing a C source.
    pub fn compile_string(mut self, source: &CStr) -> Result<Self, Error> {
        *self.errors = None;
        let ret = unsafe { bindings::tcc_compile_string(self.inner, source.as_ptr()) };
        if ret >= 0 {
            Ok(self)
        } else {
            let msgs = self.errors.take().unwrap();
            Err(Error::TCCError { msgs })
        }
    }

    /// output an executable, library or object file.
    pub fn output_file<T: AsRef<Path>>(mut self, filename: T) -> Result<(), Error> {
        *self.errors = None;
        let filename = to_cstr(filename);
        let ret = unsafe { bindings::tcc_output_file(self.inner, filename.as_ptr()) };
        if ret >= 0 {
            Ok(())
        } else {
            let msgs = self.errors.take().unwrap();
            Err(Error::TCCError { msgs })
        }
    }

    /// link and run main() function and return its value.
    ///
    /// # Safety
    /// argc and argv must be valid.
    pub unsafe fn run_unsafe(
        mut self,
        argc: c_int,
        argv: *mut *mut c_char,
    ) -> Result<c_int, Error> {
        *self.errors = None;
        let ret = unsafe { bindings::tcc_run(self.inner, argc, argv) };
        if ret >= 0 {
            Ok(ret)
        } else {
            let msgs = self.errors.take().unwrap();
            Err(Error::TCCError { msgs })
        }
    }

    /// link and run main() function and return its value.
    pub fn run(self, args: &[&str]) -> Result<c_int, Error> {
        let mut args = args
            .iter()
            .map(|s| CString::new(*s).unwrap())
            .collect::<Vec<_>>();
        let mut argv = Vec::with_capacity(args.len() + 1);
        for arg in args.iter_mut() {
            argv.push(arg.as_ptr() as *mut _);
        }
        argv.push(std::ptr::null_mut());
        unsafe { self.run_unsafe(argv.len() as c_int, argv.as_mut_ptr()) }
    }

    /// do all relocations
    pub fn relocate(mut self) -> Result<RelocatedContext, Error> {
        *self.errors = None;

        // pass null ptr to get required length
        let len = unsafe { bindings::tcc_relocate(self.inner, std::ptr::null_mut()) };
        if len == -1 {
            return Err(Error::RelocationFailed);
        };

        let mut buffer = Vec::with_capacity(len as usize);
        let ret = unsafe { bindings::tcc_relocate(self.inner, buffer.as_mut_ptr() as *mut c_void) };
        if ret != 0 {
            return Err(Error::RelocationFailed);
        }
        unsafe { buffer.set_len(len as usize) };

        Ok(RelocatedContext {
            inner: self.inner,
            _buffer: buffer,
        })
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { bindings::tcc_delete(self.inner) };
        INITIALIZED.store(false, Ordering::Relaxed);
    }
}

/// relocated context
pub struct RelocatedContext {
    inner: *mut bindings::TCCState,
    _buffer: Vec<u8>,
}

impl RelocatedContext {
    /// get a pointer to a generated function
    pub fn get_symbol(&self, name: &CStr) -> Option<Symbol> {
        let addr = unsafe { bindings::tcc_get_symbol(self.inner, name.as_ptr()) };
        let addr = NonNull::new(addr)?;
        Some(unsafe { Symbol::new(addr) })
    }

    /// list all symbols
    pub fn list_symbols(&self) -> Vec<(&CStr, Symbol)> {
        let mut symbols: Vec<(&CStr, Symbol)> = Vec::new();

        extern "C" fn symbol_callback(ctx: *mut c_void, name: *const c_char, val: *const c_void) {
            let ctx = ctx as *mut Vec<(&CStr, Symbol)>;
            let name = unsafe { CStr::from_ptr(name) };
            unsafe { (*ctx).push((name, Symbol::new(NonNull::new_unchecked(val as *mut _)))) };
        }

        unsafe {
            bindings::tcc_list_symbols(
                self.inner,
                &mut symbols as *mut _ as *mut _,
                Some(symbol_callback),
            )
        };

        symbols
    }
}

/// a symbol
pub struct Symbol<'a> {
    inner: NonNull<c_void>,
    _marker: PhantomData<&'a Context>,
}

impl<'a> Symbol<'a> {
    /// make a symbol from a pointer
    ///
    /// # Safety
    /// Pointer must be valid throughout the lifetime of the symbol.
    pub unsafe fn new(val: NonNull<c_void>) -> Self {
        Self {
            inner: val,
            _marker: PhantomData,
        }
    }

    /// get a pointer to the symbol
    pub fn as_ptr(&self) -> *const c_void {
        self.inner.as_ptr()
    }

    /// cast the symbol to a function reference
    ///
    /// # Safety
    /// The type must be correct.
    pub unsafe fn cast<T>(&self) -> &'a T {
        &*(self.as_ptr() as *const T)
    }
}

#[cfg(target_family = "unix")]
#[inline]
fn to_cstr<T: AsRef<Path>>(p: T) -> CString {
    use std::os::unix::ffi::OsStrExt;
    CString::new(p.as_ref().as_os_str().as_bytes()).unwrap()
}

#[cfg(not(target_family = "unix"))]
#[inline]
fn to_cstr<T: AsRef<Path>>(p: T) -> CString {
    CString::new(p.as_ref().to_string_lossy().to_string().as_bytes()).unwrap()
}
