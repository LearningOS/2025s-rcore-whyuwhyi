//! Process management syscalls
use crate::{
    config::PAGE_SIZE,
    mm::{translated_byte_buffer, *},
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next, get_syscall_times,
        mmap_area, munmap_area, suspend_current_and_run_next,
    },
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");

    let time = translated_byte_buffer(
        current_user_token(),
        ts as *const u8,
        core::mem::size_of::<TimeVal>(),
    );
    if time.is_empty() {
        -1
    } else {
        let sys_time = get_time_us();
        let sec = sys_time / 1_000_000;
        let usec = sys_time % 1_000_000;

        let sec_bytes = (sec as isize).to_ne_bytes();
        let usec_bytes = (usec as isize).to_ne_bytes();
        let data = [sec_bytes, usec_bytes].concat();

        let mut offset = 0;

        for word in time {
            for byte in word {
                *byte = data[offset];
                offset += 1;
            }
        }

        0
    }
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");

    let pagetable = PageTable::from_token(current_user_token());
    let vaddr = VirtAddr::from(id);
    let vpn = VirtAddr::from(id).floor();

    match trace_request {
        0 => match pagetable.translate(vpn) {
            Some(pte) if pte.is_valid() && pte.readable() && pte.user() => {
                let ppn = pte.ppn();
                let offset = vaddr.page_offset();
                let page_ptr = ppn.get_bytes_array().as_ptr();
                let byte = unsafe { *page_ptr.add(offset) };
                byte as isize
            }
            _ => -1,
        },
        1 => match pagetable.translate(vpn) {
            Some(pte) if pte.is_valid() && pte.writable() && pte.user() => {
                let ppn = pte.ppn();
                let offset = vaddr.page_offset();
                let page_ptr = ppn.get_bytes_array().as_mut_ptr();
                unsafe {
                    *page_ptr.add(offset) = data as u8;
                }
                0
            }
            _ => -1,
        },
        2 => get_syscall_times(id) as isize,
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    if start % PAGE_SIZE != 0 || (prot & !0x7 != 0) || (prot & 0x7 == 0) {
        return -1;
    }
    if len == 0 {
        return 0;
    }

    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + len);
    let mut map_perm = MapPermission::from_bits((prot << 1) as u8).unwrap();
    map_perm |= MapPermission::U;

    match mmap_area(start_va, end_va, map_perm) {
        Ok(()) => 0,
        Err(()) => -1,
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }

    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + len);

    match munmap_area(start_va, end_va) {
        Ok(()) => 0,
        Err(()) => -1,
    }
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
