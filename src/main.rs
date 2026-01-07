use colorant_rust::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use tokio::time::sleep;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Console::*;
use colorant_rust::spawn_fov_window;
use log::LevelFilter;

// Hide console window
fn hide_console() {
    unsafe {
        let console_window = GetConsoleWindow();
        if console_window.0 != 0 {
            ShowWindow(console_window, SW_HIDE);
        }
    }
}

// Setup logging to file for debugging
fn setup_logging() {
    env_logger::builder()
        .filter_level(LevelFilter::Warn) // Only show warnings and errors
        .format(|buf, record| {
            use std::io::Write;
            writeln!(buf, "[{}] {}: {}", 
                chrono::Local::now().format("%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .target(env_logger::Target::Pipe(Box::new(std::fs::File::create("colorant.log").unwrap())))
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Hide console window
    hide_console();
    
    // Setup logging
    setup_logging();
    
    // Spoof process name
    //process::spoof_process_name("svchost.exe");
    
    // Create configuration
    let mut config = Config::default();
    config.x = 100;  // Adjust based on your game window position
    config.y = 100;  // Adjust based on your game window position
    config.x_fov = 75;
    config.y_fov = 75;
    config.calculate_speeds();
    
    println!("==========================================");
    println!("🎮 Colorant Rust v{}", env!("CARGO_PKG_VERSION"));
    println!("==========================================");
    
    println!("📋 Configuration:");
    println!("   FOV: {}x{}", config.x_fov, config.y_fov);
    println!("   Sensitivity: {:.2}", config.ingame_sensitivity);
    println!("   Move Speed: {:.3}", config.move_speed);
    println!("   Flick Speed: {:.3}", config.flick_speed);
    
    // Create engine
    let engine = Arc::new(Mutex::new(ColorantEngine::new(config).await?));

    let frame_handle = {
        // If you have direct access to capture:
        // capture.get_frame_handle()
        
        // If capture is inside engine:
        let engine_lock = engine.lock().await;
        engine_lock.get_capture_frame_handle() // You'll need to add this method
    };
    
    spawn_fov_window(frame_handle);

    println!("\n🎯 COLORANT SYSTEM ACTIVE");
    println!("=========================");
    println!("✅ Arduino connected");
    println!("🎮 Press F1 to toggle aimbot");
    println!("🎮 Hold movement keys when aimbot is ON");
    println!("🎮 Press Ctrl+C to exit");
    println!("\n[SYSTEM] Starting monitoring loop...");

    {
        let mut engine_lock = engine.lock().await;
        engine_lock.toggle(); // Enable for testing
        // Quick test capture
        println!("\n🔍 Running color detection test...");
        if let Err(e) = engine_lock.process_action(Action::Move).await {
            println!("[TEST] Initial test failed: {}", e);
        } else {
            println!("[TEST] Initial test completed");
        }
        
        engine_lock.toggle(); // Disable after test
    }
    
    
    // Create hotkey manager
    //let hotkey_config = hotkey::HotkeyConfig::default();
    /*let hotkey_manager = hotkey::HotkeyManager::new(hotkey_config);
    
    // Setup F1 toggle hotkey
    hotkey_manager.register_hotkey(VK_F1, {
        let engine_clone = Arc::clone(&engine);
        move || {
            let engine = engine_clone.clone();
            tokio::spawn(async move {
                let mut engine_lock = engine.lock().await;
                let enabled = engine_lock.toggle();
                println!("[HOTKEY] F1 pressed - Aimbot: {}", 
                    if enabled { "✅ ENABLED" } else { "⏸️  DISABLED" });
            });
        }
    });
    */
    
    
    // Main monitoring loop
    let mut last_f1_state = false;
    let mut last_f5_state = false;
    let mut last_f2_state = false;
    
    println!("[SYSTEM] Starting FOV preview window...");
    match colorant_rust::run_fov_window_blocking(frame_handle) {    
    Ok(_) => println!("[FOV] Window closed normally"),
    Err(e) => eprintln!("[FOV] Window error: {}", e),
}