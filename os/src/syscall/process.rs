//! Process management syscalls
use crate::{
    config::PAGE_SIZE,
    mm::translated_byte_buffer,
    mm::*,
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next, get_syscall_times,
        suspend_current_and_run_next,
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

    let page_table = PageTable::from_token(current_user_token());
    let start_vpn = VirtAddr::from(ts as usize).floor();
    let end_vpn = VirtAddr::from(ts as usize + core::mem::size_of::<TimeVal>()).ceil();

    for vpn in VPNRange::new(start_vpn, end_vpn) {
        match page_table.translate(vpn) {
            Some(pte) if pte.is_valid() && pte.writable() => {}
            _ => return -1,
        }
    }

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

    match trace_request {
        0 => {
            let pagetable = PageTable::from_token(current_user_token());
            let vpn = VirtAddr::from(id).floor();

            match pagetable.translate(vpn) {
                Some(pte) if pte.is_valid() && pte.readable() => {}
                _ => return -1,
            }

            let buffer = translated_byte_buffer(current_user_token(), id as *const u8, 1);
            if let Some(slice) = buffer.first() {
                slice.first().copied().map_or(-1, |byte| byte as isize)
            } else {
                -1
            }
        }
        1 => {
            let pagetable = PageTable::from_token(current_user_token());
            let vpn = VirtAddr::from(id).floor();

            match pagetable.translate(vpn) {
                Some(pte) if pte.is_valid() && pte.writable() => {}
                _ => return -1,
            }

            let mut buffer = translated_byte_buffer(current_user_token(), id as *const u8, 1);

            if let Some(slice) = buffer.first_mut() {
                if let Some(byte) = slice.first_mut() {
                    *byte = data as u8;
                    return 0;
                }
            }
            -1
        }
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

    let token = current_user_token();
    let mut pagetable = PageTable::from_token(token);

    let start_vpn = VirtAddr::from(start).floor();
    let end_vpn = VirtAddr::from(start + len).ceil();

    for vpn in VPNRange::new(start_vpn, end_vpn) {
        match pagetable.translate(vpn) {
            Some(pte) if pte.is_valid() => return -1,
            _ => {}
        }
    }

    let mut flags = PTEFlags::empty();
    if prot & 0x1 != 0 {
        flags |= PTEFlags::R;
    }
    if prot & 0x2 != 0 {
        flags |= PTEFlags::W;
    }
    if prot & 0x4 != 0 {
        flags |= PTEFlags::X;
    }
    flags |= PTEFlags::U;

    for vpn in VPNRange::new(start_vpn, end_vpn) {
        let frame = match frame_alloc() {
            Some(frame) => frame,
            None => return -1,
        };
        pagetable.map(vpn, frame.ppn, flags);
    }
    0
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

    let token = current_user_token();
    let mut pagetable = PageTable::from_token(token);

    let start_vpn = VirtAddr::from(start).floor();
    let end_vpn = VirtAddr::from(start + len).ceil();

    for vpn in VPNRange::new(start_vpn, end_vpn) {
        match pagetable.translate(vpn) {
            Some(pte) if pte.is_valid() => {}
            _ => return -1,
        }
    }

    for vpn in VPNRange::new(start_vpn, end_vpn) {
        pagetable.unmap(vpn);
    }
    0
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
