use winit::event::{ElementState, KeyboardInput, ScanCode};

const W_SCANCODE: ScanCode = 17;
const A_SCANCODE: ScanCode = 30;
const S_SCANCODE: ScanCode = 31;
const D_SCANCODE: ScanCode = 32;
const SPACE_SCANCODE: ScanCode = 57;
const C_SCANCODE: ScanCode = 46;
const UP_SCANCODE: ScanCode = 57416;
const LEFT_SCANCODE: ScanCode = 57419;
const DOWN_SCANCODE: ScanCode = 57424;
const RIGHT_SCANCODE: ScanCode = 57421;

#[derive(Default, Debug)]
pub struct ControllerState {
    forward_pressed: bool,
    backward_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
    jump_pressed: bool,
    crouch_pressed: bool,
}

impl ControllerState {
    pub fn new() -> ControllerState {
        Self::default()
    }

    pub fn handle_keyboard_event(&mut self, input: &KeyboardInput) -> bool {
        dbg!(input);
        match input.state {
            ElementState::Pressed => match input.scancode {
                W_SCANCODE | UP_SCANCODE => {
                    self.forward_pressed = true;
                    true
                }
                A_SCANCODE | LEFT_SCANCODE => {
                    self.left_pressed = true;
                    true
                }
                S_SCANCODE | DOWN_SCANCODE => {
                    self.backward_pressed = true;
                    true
                }
                D_SCANCODE | RIGHT_SCANCODE => {
                    self.right_pressed = true;
                    true
                }
                SPACE_SCANCODE => {
                    self.jump_pressed = true;
                    true
                }
                C_SCANCODE => {
                    self.crouch_pressed = true;
                    true
                }
                _ => false,
            },
            ElementState::Released => match input.scancode {
                W_SCANCODE | UP_SCANCODE => {
                    self.forward_pressed = false;
                    true
                }
                A_SCANCODE | LEFT_SCANCODE => {
                    self.left_pressed = false;
                    true
                }
                S_SCANCODE | DOWN_SCANCODE => {
                    self.backward_pressed = false;
                    true
                }
                D_SCANCODE | RIGHT_SCANCODE => {
                    self.right_pressed = false;
                    true
                }
                SPACE_SCANCODE => {
                    self.jump_pressed = false;
                    true
                }
                C_SCANCODE => {
                    self.crouch_pressed = false;
                    true
                }
                _ => false,
            },
        }
    }

    pub fn forward_multiplier(&self) -> f32 {
        (self.forward_pressed as i32 - self.backward_pressed as i32) as f32
    }

    pub fn right_muliplier(&self) -> f32 {
        (self.right_pressed as i32 - self.left_pressed as i32) as f32
    }

    pub fn jump_multiplier(&self) -> f32 {
        (self.jump_pressed as i32 - self.crouch_pressed as i32) as f32
    }
}