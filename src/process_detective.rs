#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::collections::HashMap;
use std::ffi::c_void;
use std::mem;
use std::ptr::null_mut;

const MAX_PATH: usize = 260;
const TH32CS_SNAPPROCESS: u32 = 0x00000002;
const TH32CS_SNAPMODULE: u32 = 0x00000008;
const TH32CS_SNAPMODULE32: u32 = 0x00000010;
const INVALID_HANDLE_VALUE: *mut c_void = -1isize as *mut c_void;
const PROCESS_VM_READ: u32 = 0x0010;
const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
const STILL_ACTIVE: u32 = 259;

#[repr(C)]
pub struct PROCESSENTRY32W {
    pub dwSize: u32,
    pub cntUsage: u32,
    pub th32ProcessID: u32,
    pub th32DefaultHeapID: usize,
    pub th32ModuleID: u32,
    pub cntThreads: u32,
    pub th32ParentProcessID: u32,
    pub pcPriClassBase: i32,
    pub dwFlags: u32,
    pub szExeFile: [u16; MAX_PATH],
}

#[repr(C)]
pub struct MODULEENTRY32W {
    pub dwSize: u32,
    pub th32ModuleID: u32,
    pub th32ProcessID: u32,
    pub GlblcntUsage: u32,
    pub ProccntUsage: u32,
    pub modBaseAddr: *mut u8,
    pub modBaseSize: u32,
    pub hModule: *mut c_void,
    pub szModule: [u16; 256],
    pub szExePath: [u16; MAX_PATH],
}

extern "system" {
    fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> *mut c_void;
    fn Module32FirstW(hSnapshot: *mut c_void, lpme: *mut MODULEENTRY32W) -> i32;
    fn Module32NextW(hSnapshot: *mut c_void, lpme: *mut MODULEENTRY32W) -> i32;
    fn Process32FirstW(hSnapshot: *mut c_void, lppe: *mut PROCESSENTRY32W) -> i32;
    fn Process32NextW(hSnapshot: *mut c_void, lppe: *mut PROCESSENTRY32W) -> i32;
    fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> *mut c_void;
    fn ReadProcessMemory(
        hProcess: *mut c_void,
        lpBaseAddress: *const c_void,
        lpBuffer: *mut c_void,
        nSize: usize,
        lpNumberOfBytesRead: *mut usize,
    ) -> i32;
    fn CloseHandle(hObject: *mut c_void) -> i32;
    fn GetExitCodeProcess(hProcess: *mut c_void, lpExitCode: *mut u32) -> i32;
}

pub trait GameModule {
    fn name(&self) -> &'static str;
}

#[derive(Default, Clone)]
pub struct ProcessDetective {
    process: Option<*mut c_void>,
    game_modules: HashMap<String, usize>,
}

impl ProcessDetective {
    pub fn new() -> Self {
        Self {
            process: None,
            game_modules: HashMap::new(),
        }
    }

    fn collect_all_modules(&mut self, pid: u32) -> bool {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid);
            if snapshot == INVALID_HANDLE_VALUE {
                return false;
            }

            let mut module = MODULEENTRY32W {
                dwSize: size_of::<MODULEENTRY32W>() as u32,
                th32ModuleID: 0,
                th32ProcessID: 0,
                GlblcntUsage: 0,
                ProccntUsage: 0,
                modBaseAddr: null_mut(),
                modBaseSize: 0,
                hModule: null_mut(),
                szModule: [0; 256],
                szExePath: [0; MAX_PATH],
            };

            if Module32FirstW(snapshot, &mut module) != 0 {
                loop {
                    let name_utf16: Vec<u16> = module
                        .szModule
                        .iter()
                        .take_while(|&&c| c != 0)
                        .copied()
                        .collect();
                    let name = String::from_utf16_lossy(&name_utf16);

                    self.game_modules.insert(name, module.modBaseAddr as usize);

                    if Module32NextW(snapshot, &mut module) == 0 {
                        break;
                    }
                }
            }

            CloseHandle(snapshot);
            true
        }
    }

    pub fn find_process(&mut self, name: &str) -> Result<(), &str> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == (-1isize as *mut c_void) {
                return Err("CreateToolhelp32Snapshot return INVALID_HANDLE_VALUE");
            }

            let mut entry = PROCESSENTRY32W {
                dwSize: size_of::<PROCESSENTRY32W>() as u32,
                cntUsage: 0,
                th32ProcessID: 0,
                th32DefaultHeapID: 0,
                th32ModuleID: 0,
                cntThreads: 0,
                th32ParentProcessID: 0,
                pcPriClassBase: 0,
                dwFlags: 0,
                szExeFile: [0; MAX_PATH],
            };

            if Process32FirstW(snapshot, &mut entry) != 0 {
                loop {
                    let exe_name = String::from_utf16_lossy(
                        &entry
                            .szExeFile
                            .iter()
                            .take_while(|&&c| c != 0)
                            .cloned()
                            .collect::<Vec<_>>(),
                    );

                    if exe_name == name {
                        let handle = OpenProcess(
                            PROCESS_VM_READ | PROCESS_QUERY_INFORMATION,
                            0,
                            entry.th32ProcessID,
                        );

                        CloseHandle(snapshot);

                        if !handle.is_null() && self.collect_all_modules(entry.th32ProcessID) {
                            self.process = Some(handle);
                        } else {
                            return Err("Error opening the process,\
                                 admin privileges are needed or the process is protected");
                        }

                        return Ok(());
                    }

                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }

            CloseHandle(snapshot);
        }

        Err("There is no process with that name")
    }

    pub fn is_active(&self) -> bool {
        self.process.map_or(false, |handle| unsafe {
            let mut exit_code: u32 = 0;
            let result = GetExitCodeProcess(handle, &mut exit_code);

            result != 0 && exit_code == STILL_ACTIVE
        })
    }

    pub fn read_mem<T: Sized>(&mut self, gm: &dyn GameModule, offset: usize) -> Option<T> {
        let addr = *self.game_modules.get(gm.name()).unwrap_or(&0);

        if addr == 0 {
            return None;
        }

        self.process.map_or(None, |p| unsafe {
            let mut local: T = mem::zeroed();
            let mut bytes_read: usize = 0;

            let success = ReadProcessMemory(
                p,
                (addr + offset) as *const c_void,
                &mut local as *mut _ as *mut c_void,
                size_of::<T>(),
                &mut bytes_read,
            );

            if success != 0 && bytes_read == size_of::<T>() {
                Some(local)
            } else {
                None
            }
        })
    }
}

pub trait GameObject {
    fn new() -> Self;
    fn read(&mut self, pd: &mut ProcessDetective);
}
