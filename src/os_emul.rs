/// OsEmul module provides path resolution via cygwin1.dll
/// (which must be present in PATH).

#[cfg(windows)]
extern crate kernel32;
#[cfg(windows)]
extern crate winapi;

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::iter::once;
use std::path::{Path,PathBuf};
use std::vec::Vec;
use std::mem;

use winapi::winnt::{
    FILE_ATTRIBUTE_SYSTEM,
    FILE_ATTRIBUTE_READONLY,
};

use DirEntry;

pub const CCP_POSIX_TO_WIN_W : isize = 1;
pub const CCP_WIN_W_TO_POSIX : isize = 3;

// Stub

#[cfg(not(windows))]
pub struct OsEmul {
}

#[cfg(not(windows))]
impl OsEmul {
    fn new() -> OsEmul {
        return OsEmul {
        };
    }

    fn running_under_cygwin(&self) -> bool {
        return false;
    }
}

// Implementation

#[cfg(windows)]
pub struct OsEmul {
    cygwin_dll: winapi::HINSTANCE,
    cygwin_conv_path: winapi::FARPROC,
}

#[cfg(windows)]
impl OsEmul {

    /// Performs runtime linking to cygwin1.dll if it is present
    pub fn new() -> OsEmul {
        let cygwin_dll_z = "cygwin1.dll\0".as_bytes().as_ptr() as *const i8;
        let cygwin_dll = unsafe {
            kernel32::LoadLibraryA(cygwin_dll_z)
        };
        println!(" cyg dll = {:?}", cygwin_dll);

        let conv_path_z = "cygwin_conv_path\0".as_bytes().as_ptr() as *const i8;
        let cygwin_conv_path = unsafe {
            kernel32::GetProcAddress(cygwin_dll, conv_path_z)
        };
        println!(" conv path = {:?}", cygwin_conv_path);

        return OsEmul {
            cygwin_dll: cygwin_dll,
            cygwin_conv_path: cygwin_conv_path,
        };
    }

    /// Cheap to call
    pub fn running_under_cygwin(&self) -> bool {
        return (self.cygwin_dll as usize) != 0;
    }

    pub fn convert_cygpath_to_win(&self, path: &Path) -> PathBuf {
        let mode = CCP_POSIX_TO_WIN_W;
        self.cygwin_convert_path(mode, path)
    }

    pub fn convert_winpath_to_cyg(&self, path: &Path) -> PathBuf {
        let mode = CCP_WIN_W_TO_POSIX;
        self.cygwin_convert_path(mode, path)
    }

    pub fn cygwin_convert_path(&self, mode: isize, path: &Path) -> PathBuf {
        let path_s = path.to_string_lossy();
        let path_z = format!("{}\0", path_s);
        let path_zb = path_z.as_bytes().as_ptr();

        unsafe {
            let conv_path : fn(isize, *const u8, *mut u8, isize) -> isize =
                mem::transmute(self.cygwin_conv_path);
            // Get resulting path's length
            let sz : isize = conv_path(mode, path_zb, 0 as *mut u8, 0);
            if sz < 0 {
                let err_path = format!("::CYGWIN::INVALID_PATH:: {}", path_s);
                return PathBuf::from(err_path);
            }
            // Allocate memory for path
            let mut out_path : Vec<u8> = Vec::with_capacity(sz as usize);
            // Receive the path
            conv_path(mode, path_zb, (&out_path[0..]).as_ptr() as *mut u8, sz);
            // Covnert the path into PathBuf
            let out_path_utf8 = String::from_utf8_lossy(&out_path[0..]).into_owned();
            return PathBuf::from(out_path_utf8);
        }
    }

    pub fn path_is_probably_cygwin_symlink(&self, path: &Path) -> bool {
        let path_s = path.to_string_lossy().into_owned();
        let path_wz: Vec<u16> = OsStr::new(&path_s).encode_wide().chain(once(0)).collect();
        let attr = unsafe { kernel32::GetFileAttributesW(path_wz.as_ptr()) };
        return ((attr & FILE_ATTRIBUTE_SYSTEM) != 0)
            || (((attr & FILE_ATTRIBUTE_READONLY) != 0) && path_s.as_str().ends_with(".lnk"));
    }

/*
    // stdlib as of rustc 1.14.0 misses Metadata::systemfile()
    pub fn dirent_is_probably_cygwin_symlink(&self, dent: DirEntry) -> bool {
        let path_s = dent.path().to_string_lossy();
        let meta = dent.metadata();
        return meta.systemfile()
            || (meta.readonly() && path_s.as_str().ends_with(".lnk"));
    }
*/
    
    pub fn dirent_is_probably_cygwin_symlink(&self, dent: &DirEntry) -> bool {
        self.path_is_probably_cygwin_symlink(dent.path())
    }

    pub fn dereference_cygwin_symlink(&self, path: &Path) -> PathBuf {
        let cygwin_path = self.convert_winpath_to_cyg(path);
        self.convert_cygpath_to_win(cygwin_path.as_path())
    }

}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}

