// src/capture.rs - CORRECTED WORKING VERSION
use anyhow::Result;
use image::RgbImage;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    CreateDCW, DeleteDC, BitBlt, GetDIBits, BITMAPINFO, BITMAPINFOHEADER,
    GetDC, ReleaseDC, CreateCompatibleDC,
    RGBQUAD, DIB_RGB_COLORS, SRCCOPY, BI_RGB
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
use windows::Win32::Foundation::{HWND, GetLastError};

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
                    // Use GetDC instead of CreateDCW for screen capture
                    use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
                    use windows::Win32::Foundation::HWND;
                    
                    let hwnd = HWND(0); // Desktop window
                    let hdc_screen = GetDC(hwnd);
                    
                    if hdc_screen.is_null() {
                        eprintln!("[CAPTURE] Failed to get screen DC");
                        std::thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                    
                    let hdc_mem = windows::Win32::Graphics::Gdi::CreateCompatibleDC(hdc_screen);
                    
                    if hdc_mem.is_null() {
                        eprintln!("[CAPTURE] Failed to create compatible DC");
                        ReleaseDC(hwnd, hdc_screen);
                        std::thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                    
                    // Create bitmap
                    let hbitmap = windows::Win32::Graphics::Gdi::CreateCompatibleBitmap(
                        hdc_screen,
                        config.width as i32,
                        config.height as i32
                    );
                    
                    if hbitmap.is_null() {
                        eprintln!("[CAPTURE] Failed to create bitmap");
                        windows::Win32::Graphics::Gdi::DeleteDC(hdc_mem);
                        ReleaseDC(hwnd, hdc_screen);
                        std::thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                    
                    // Select bitmap into memory DC
                    let _old_bitmap = windows::Win32::Graphics::Gdi::SelectObject(hdc_mem, hbitmap);
                    
                    // Debug: Print what we're trying to capture
                    println!("[CAPTURE] Capturing region: x={}, y={}, {}x{}", 
                        config.x, config.y, config.width, config.height);
                    
                    // Copy screen region - FIXED: Use the correct BitBlt function
                    let success = windows::Win32::Graphics::Gdi::BitBlt(
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
                    
                    if !success.as_bool() {
                        let error = windows::Win32::Foundation::GetLastError();
                        eprintln!("[CAPTURE] BitBlt failed with error: {:?}", error);
                    }
                    
                    // Get bitmap data
                    let mut bmi = BITMAPINFO {
                        bmiHeader: BITMAPINFOHEADER {
                            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                            biWidth: config.width as i32,
                            biHeight: -(config.height as i32), // Negative for top-down
                            biPlanes: 1,
                            biBitCount: 24,
                            biCompression: BI_RGB.0,
                            biSizeImage: 0,
                            biXPelsPerMeter: 0,
                            biYPelsPerMeter: 0,
                            biClrUsed: 0,
                            biClrImportant: 0,
                        },
                        bmiColors: [RGBQUAD { rgbBlue: 0, rgbGreen: 0, rgbRed: 0, rgbReserved: 0 }; 1],
                    };
                    
                    let mut buffer = vec![0u8; (config.width * config.height * 3) as usize];
                    
                    let lines_copied = GetDIBits(
                        hdc_mem,
                        hbitmap,
                        0,
                        config.height as u32,
                        Some(buffer.as_mut_ptr() as *mut std::ffi::c_void),
                        &mut bmi,
                        DIB_RGB_COLORS
                    );
                    
                    // Debug: Check if we got data
                    if lines_copied > 0 {
                        // Check first pixel
                        if buffer.len() >= 3 {
                            println!("[CAPTURE DEBUG] First pixel BGR: ({}, {}, {})", 
                                buffer[0], buffer[1], buffer[2]);
                        }
                    } else {
                        eprintln!("[CAPTURE] GetDIBits failed, copied {} lines", lines_copied);
                    }
                    
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
                        
                        // Debug: Check image stats
                        let avg_color: (u32, u32, u32) = image.pixels()
                            .fold((0, 0, 0), |(r, g, b), pixel| 
                                (r + pixel[0] as u32, g + pixel[1] as u32, b + pixel[2] as u32));
                        let pixel_count = config.width * config.height;
                        println!("[CAPTURE DEBUG] Average color: ({}, {}, {})", 
                            avg_color.0 / pixel_count, 
                            avg_color.1 / pixel_count, 
                            avg_color.2 / pixel_count);
                    } else {
                        eprintln!("[CAPTURE] Failed to create image from buffer");
                    }
                    
                    // Cleanup
                    windows::Win32::Graphics::Gdi::DeleteObject(hbitmap);
                    windows::Win32::Graphics::Gdi::DeleteDC(hdc_mem);
                    ReleaseDC(hwnd, hdc_screen);
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