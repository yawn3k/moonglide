use std::time::{Duration, Instant};

#[cfg(windows)]
use windows_sys::Win32::System::Threading::{
    CreateWaitableTimerExW, SetWaitableTimerEx, WaitForSingleObject, SetThreadPriority,
    GetCurrentThread, CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, THREAD_PRIORITY_HIGHEST,
};
#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, INFINITE, HANDLE};

pub(crate) struct FramePacer {
    frame_dur: Duration,
    next_frame: Instant,
    #[cfg(windows)]
    timer: HANDLE,
}

impl FramePacer {
    pub(crate) fn new(fps: f64) -> Self {
        let frame_dur = Duration::from_secs_f64(1.0 / fps);
        let next_frame = Instant::now();

        #[cfg(windows)]
        let timer = unsafe {
            let h = CreateWaitableTimerExW(
                std::ptr::null(),
                std::ptr::null(),
                CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                0x1F0003,
            );
            if !h.is_null() {
                SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);
            }
            h
        };

        FramePacer {
            frame_dur,
            next_frame,
            #[cfg(windows)]
            timer,
        }
    }

    pub(crate) fn wait(&mut self) {
        self.next_frame += self.frame_dur;
        let now = Instant::now();
        if now < self.next_frame {
            let remaining = self.next_frame - now;

            #[cfg(windows)]
            {
                if !self.timer.is_null() {
                    let due: i64 = -(remaining.as_nanos() as i64 / 100);
                    unsafe {
                        SetWaitableTimerEx(
                            self.timer,
                            &due,
                            0,
                            None,
                            std::ptr::null(),
                            std::ptr::null(),
                            0,
                        );
                        WaitForSingleObject(self.timer, INFINITE);
                    }
                    return;
                }
            }

            std::thread::sleep(remaining);
        }
    }
}

#[cfg(windows)]
impl Drop for FramePacer {
    fn drop(&mut self) {
        if !self.timer.is_null() {
            unsafe {
                CloseHandle(self.timer);
            }
        }
    }
}
