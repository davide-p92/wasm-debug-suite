#[unsafe(no_mangle)]
pub static mut MEMORY_BUFFER: [u8; 8] = [0; 8];

#[unsafe(no_mangle)]
pub static mut GLOBAL_COUNTER: i32 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn _step() {
    unsafe {
        GLOBAL_COUNTER += 1;
        MEMORY_BUFFER[0] = (GLOBAL_COUNTER & 0xFF) as u8;
        MEMORY_BUFFER[1] = ((GLOBAL_COUNTER >> 8) & 0xFF) as u8;
    }
}
/*
#[unsafe(no_mangle)]
pub extern "C" fn get_counter() -> i32 {
    unsafe {
        GLOBAL_COUNTER
    }
}*/
