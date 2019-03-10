pub struct CriticalSectionGuard {}

impl Drop for CriticalSectionGuard {
    fn drop(&mut self) {
        unsafe { boinc_end_critical_section(); }
    }
}

impl CriticalSectionGuard {
    fn new() -> Self {
        unsafe { boinc_begin_critical_section(); }
        Self {}
    }
}