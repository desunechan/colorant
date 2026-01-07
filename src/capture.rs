// src/capture.rs - CORRECTED WORKING VERSION
use anyhow::Result;
use image::RgbImage;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    CreateDCW, DeleteDC, BitBlt, GetDIBits, BITMAPINFO, BITMAPINFOHEADER, 
    RGBQUAD, DIB_RGB_COLORS, SRCCOPY, BI_RGB
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

#[derive(Debug, Clone, Copy)]
pub struct CaptureConfig {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }
    }
}

pub struct Capture {
    config: CaptureConfig,
    latest_frame: Arc<Mutex<Option<RgbImage>>>,
    paused: Arc<Mutex<bool>>,
    running: Arc<Mutex<bool>>,
}

impl Capture {
    pub fn new(config: CaptureConfig) -> Result<Self> {
        // Debug prints are correctly inside the function
        println!("[CAPTURE] Attempting capture at x:{}, y:{}, {}x{}",
            config.x, config.y, config.width, config.height);
        println!("[CAPTURE] Screen dimensions: {:?}",
            (unsafe { GetSystemMetrics(SM_CXSCREEN) }, unsafe { GetSystemMetrics(SM_CYSCREEN) }));
        
        let capture = Self {
            config,
            latest_frame: Arc::new(Mutex::new(None)),
            paused: Arc::new(Mutex::new(false)),
            running: Arc::new(Mutex::new(true)),
        };
        
        capture.start_capture_thread();
        
        Ok(capture)
    }
    
    pub fn get_frame_handle(&self) -> Arc<Mutex<Option<RgbImage>>> {
        Arc::clone(&self.latest_frame)
    }
    
    pub fn with_fov(x: i32, y: i32, width: u32, height: u32) -> Result<Self> {
        let config = CaptureConfig {
            x,
            y,
            width,
            height,
        };
        
        Self::new(config)
    }
    
    fn start_capture_thread(&self) {
        let frame_clone = Arc::clone(&self.latest_frame);
        let paused_clone = Arc::clone(&self.paused);
        let running_clone = Arc::clone(&self.running);
        let config = self.config;
        
        std::thread::spawn(move || {
            while *running_clone.lock() {
                if *paused_clone.lock() {
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                
                unsafe {
                    let hdc_screen = CreateDCW(PCWSTR::null(), PCWSTR::null(), PCWSTR::null(), None);
                    let hdc_mem = CreateDCW(PCWSTR::null(), PCWSTR::null(), PCWSTR::null(), None);
                    
                    // Create bitmap
                    let hbitmap = windows::Win32::Graphics::Gdi::CreateCompatibleBitmap(
                        hdc_screen,
                        config.width as i32,
                        config.height as i32
                    );
                    
                    // Select bitmap into memory DC
                    let _old_bitmap = windows::Win32::Graphics::Gdi::SelectObject(hdc_mem, hbitmap);
                    
                    // Copy screen region
                    let _ = BitBlt(
                        hdc_mem,
                        0,
                        0,
                        config.width as i32,
                        config.height as i32,
                        hdc_screen,
                        config.x,
                        config.y,
                        SRCCOPY
                    );
                    
                    // Get bitmap data
                    let mut bmi = BITMAPINFO {
                        bmiHeader: BITMAPINFOHEADER {
                            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                            biWidth: config.width as i32,
                            biHeight: -(config.height as i32), // Negative for top-down
                            biPlanes: 1,
                            biBitCount: 24,
                            biCompression: BI_RGB.0, // Use the constant
                            biSizeImage: 0,
                            biXPelsPerMeter: 0,
                            biYPelsPerMeter: 0,
                            biClrUsed: 0,
                            biClrImportant: 0,
                        },
                        bmiColors: [RGBQUAD { rgbBlue: 0, rgbGreen: 0, rgbRed: 0, rgbReserved: 0 }; 1],
                    };
                    
                    let mut buffer = vec![0u8; (config.width * config.height * 3) as usize];
                    
                    GetDIBits(
                        hdc_mem,
                        hbitmap,
                        0,
                        config.height as u32,
                        Some(buffer.as_mut_ptr() as *mut std::ffi::c_void),
                        &mut bmi,
                        DIB_RGB_COLORS
                    );
                    
                    // Convert BGR to RGB
                    let mut rgb_buffer = vec![0u8; (config.width * config.height * 3) as usize];
                    for i in (0..buffer.len()).step_by(3) {
                        rgb_buffer[i] = buffer[i + 2];     // R
                        rgb_buffer[i + 1] = buffer[i + 1]; // G
                        rgb_buffer[i + 2] = buffer[i];     // B
                    }
                    
                    // Create image
                    if let Some(image) = RgbImage::from_raw(config.width, config.height, rgb_buffer) {
                        *frame_clone.lock() = Some(image);
                    }
                    
                    // Cleanup
                    windows::Win32::Graphics::Gdi::DeleteObject(hbitmap);
                    DeleteDC(hdc_mem);
                    DeleteDC(hdc_screen);
                }
                
                std::thread::sleep(Duration::from_millis(10));
            }
        });
    }
    
    pub fn get_frame(&self) -> Option<RgbImage> {
        self.latest_frame.lock().clone()
    }
    
    pub fn get_frame_blocking(&self, timeout: Duration) -> Option<RgbImage> {
        let start = Instant::now();
        
        while start.elapsed() < timeout {
            if let Some(frame) = self.get_frame() {
                return Some(frame);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        
        None
    }
    
    pub fn pause(&self) {
        *self.paused.lock() = true;
    }
    
    pub fn resume(&self) {
        *self.paused.lock() = false;
    }
    
    pub fn is_paused(&self) -> bool {
        *self.paused.lock()
    }
    
    pub fn stop(&self) {
        *self.running.lock() = false;
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.stop();
    }
}