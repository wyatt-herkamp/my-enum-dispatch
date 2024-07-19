use my_enum_dispatch::EnumDispatch;

pub trait TestTrait {
    fn test(&self);
}
impl TestTrait for i32 {
    fn test(&self) {
        println!("test");
    }
}
impl TestTrait for f32 {
    fn test(&self) {
        println!("test");
    }
}
#[derive(EnumDispatch)]
#[enum_dispatch(TestTrait)]
#[function(fn test(&self))]
pub enum TestEnum {
    #[enum_dispatch(from)]
    A(i32),
    #[enum_dispatch(from)]
    B(f32),
    #[enum_dispatch(modifier = as_ref)]
    Any(Box<dyn TestTrait>),
}
fn main() {}
