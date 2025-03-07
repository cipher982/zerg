pub mod setup;
pub mod events;
pub mod main;

// Animation loop function
pub fn setup_animation_loop() {
    // Animation disabled to prevent RefCell borrow issues
    // The only thing this powered was the processing status visual effect
    // This is our own implementation that doesn't depend on ui_old.rs
}
