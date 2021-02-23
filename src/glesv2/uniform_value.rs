use super::*;

#[derive(Clone, Copy)]
pub enum UniformValue {
    Float(GLfloat),
    Vec2f(GLfloat, GLfloat),
    Vec3f(GLfloat, GLfloat, GLfloat),
    Vec4f(GLfloat, GLfloat, GLfloat, GLfloat),
    Floatv(GLsizei, *const GLfloat),
    Vec2fv(GLsizei, *const GLfloat),
    Vec3fv(GLsizei, *const GLfloat),
    Vec4fv(GLsizei, *const GLfloat),
    Matrix2fv(GLsizei, *const GLfloat),
    Matrix3fv(GLsizei, *const GLfloat),
    Matrix4fv(GLsizei, *const GLfloat),
    Int(GLint),
}
