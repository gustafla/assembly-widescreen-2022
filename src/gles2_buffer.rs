use opengles::glesv2::{self, GLuint, GLenum, GLintptr};

pub trait Buffer: Sized {
    const TARGET: GLenum;

    fn new() -> Self;
    fn buf(&self) -> &GLuint;

    fn bind(&self) {
        glesv2::bind_buffer(Self::TARGET, *self.buf());
    }

    fn bind_default(&self) {
        glesv2::bind_buffer(Self::TARGET, 0);
    }

    fn sub_data<T>(&self, offset: GLintptr, data: &[T]) {
        self.bind();
        glesv2::buffer_sub_data(Self::TARGET, offset, data);
    }

    fn data<T>(self, data: &[T], usage: GLenum) -> Self {
        self.bind();
        glesv2::buffer_data(Self::TARGET, data, usage);
        self
    }

    fn stream_data<T>(self, data: &[T]) -> Self {
        self.data(data, glesv2::GL_STREAM_DRAW)
    }

    fn static_data<T>(self, data: &[T]) -> Self {
        self.data(data, glesv2::GL_STATIC_DRAW)
    }

    fn dynamic_data<T>(self, data: &[T]) -> Self {
        self.data(data, glesv2::GL_DYNAMIC_DRAW)
    }
}

struct BufferHandle(GLuint);

impl BufferHandle {
    fn new() -> BufferHandle {
        let handle = glesv2::gen_buffers(1)[0];
        eprintln!("Buffer handle {} created", handle);
        BufferHandle(handle)
    }
}

impl Drop for BufferHandle {
    fn drop(&mut self) {
        eprintln!("Buffer handle {} dropped", self.0);
        glesv2::delete_buffers(&[self.0]);
    }
}

pub struct ArrayBuffer {
    handle: BufferHandle,
}

impl Buffer for ArrayBuffer {
    const TARGET: GLenum = glesv2::GL_ARRAY_BUFFER;

    fn new() -> Self {
        ArrayBuffer{handle: BufferHandle::new()}
    }

    fn buf(&self) -> &GLuint {
        &self.handle.0
    }
}

pub struct ElementArrayBuffer {
    handle: BufferHandle,
}

impl Buffer for ElementArrayBuffer {
    const TARGET: GLenum = glesv2::GL_ELEMENT_ARRAY_BUFFER;

    fn new() -> Self {
        ElementArrayBuffer{handle: BufferHandle::new()}
    }

    fn buf(&self) -> &GLuint {
        &self.handle.0
    }
}
