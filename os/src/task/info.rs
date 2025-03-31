/// Syscall info of a task

#[derive(Copy, Clone, PartialEq)]
/// syscall info of a task
pub struct SyscallInfo {
    /// syscall id
    pub id: usize,
    /// syscall times of a task
    pub times: usize,
}

impl SyscallInfo {
    /// create a new syscall info
    pub fn new(id: usize) -> Self {
        Self { id, times: 0 }
    }
    /// get the syscall id
    pub fn id(&self) -> usize {
        self.id
    }
    /// get the syscall times
    pub fn times(&self) -> usize {
        self.times
    }
    /// increase the syscall times
    pub fn increase(&mut self) {
        self.times += 1;
    }
}
