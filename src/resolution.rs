#[derive(Debug, Clone, Copy)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Into<glutin::dpi::Size> for Resolution {
    fn into(self) -> glutin::dpi::Size {
        glutin::dpi::Size::Physical(glutin::dpi::PhysicalSize::new(self.width, self.height))
    }
}

impl From<glutin::dpi::PhysicalSize<u32>> for Resolution {
    fn from(size: glutin::dpi::PhysicalSize<u32>) -> Self {
        Self {
            width: size.width,
            height: size.height,
        }
    }
}
