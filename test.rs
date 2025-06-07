use std::cell::{RefCell, RefMut};
struct AppState { val: i32 }
fn draw_nodes(state: &mut AppState) {
    state.val += 1;
}
fn main() {
    let cell = RefCell::new(AppState { val: 0 });
    let mut st: RefMut<'_, AppState> = cell.borrow_mut();
    draw_nodes(&mut st);
}
