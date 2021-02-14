extern crate glfw;
extern crate gl;

use std::ffi::{CString, CStr};
use glfw::{Action, Context, Key};

// following the tutorial from http://nercury.github.io/rust/opengl/tutorial/2018/02/10/opengl-in-rust-from-scratch-03-compiling-shaders.html

struct Shader {
    id: gl::types::GLuint,
}

impl Shader {
    fn from_source(source: &CStr, kind: gl::types::GLenum) -> Result<Shader, String> {
        let id = shader_from_source(source, kind)?;
        Ok(Shader { id })
    }

    fn from_vert_source(source: &CStr) -> Result<Shader, String> {
        Shader::from_source(source, gl::VERTEX_SHADER)
    }

    fn from_frag_source(source: &CStr) -> Result<Shader, String> {
        Shader::from_source(source, gl::FRAGMENT_SHADER)
    }

    fn id(&self) -> gl::types::GLuint {
        self.id
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.id);
        }
    }
}

struct Program {
    id: gl::types::GLuint,
}

impl Program {
    fn from_shaders(shaders: &[Shader]) -> Result<Program, String> {
        let program_id = unsafe { gl::CreateProgram() };

        for shader in shaders {
            unsafe { gl::AttachShader(program_id, shader.id()); }
        }

        unsafe { gl::LinkProgram(program_id); }

        let mut success: gl::types::GLint = 1;
        unsafe {
            gl::GetProgramiv(program_id, gl::LINK_STATUS, &mut success);
        }

        if success == 0 {
            let mut len: gl::types::GLint = 0;
            unsafe {
                gl::GetProgramiv(program_id, gl::INFO_LOG_LENGTH, &mut len);
            }

            let error = create_whitespace_cstring_with_len(len as usize);

            unsafe {
                gl::GetProgramInfoLog(
                    program_id,
                    len,
                    std::ptr::null_mut(),
                    error.as_ptr() as *mut gl::types::GLchar
                );
            }

            return Err(error.to_string_lossy().into_owned());
        }


        for shader in shaders {
            unsafe { gl::DetachShader(program_id, shader.id()); }
        }

        Ok(Program { id: program_id })
    }

    fn set_used(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}

pub fn init() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    let (mut window, events) = glfw.create_window(300, 300, "Chess", glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window.");

    window.set_key_polling(true);
    window.make_current();

    window.get_proc_address("Chess");

    let _gl = gl::load_with(|s| window.get_proc_address(s) as *const std::os::raw::c_void);

    // TODO: delete this code
    let vert_shader = Shader::from_vert_source(
        &CString::new(include_str!("white.vert")).unwrap()
    ).unwrap();

    let frag_shader = Shader::from_frag_source(
        &CString::new(include_str!("white.frag")).unwrap()
    ).unwrap();

    let shader_program = Program::from_shaders(
        &[vert_shader, frag_shader]
    ).unwrap();

    let vertices: Vec<f32> = vec![
        0.5, -0.5, 0.0,   1.0, 0.0, 0.0,
        -0.5, -0.5, 0.0,  0.0, 1.0, 0.0,
        0.0,  0.5, 0.0,   0.0, 0.0, 1.0
    ];

    let mut vbo: gl::types::GLuint = 0;
    unsafe {
        gl::GenBuffers(1, &mut vbo);
    }

    unsafe {
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER, // target
            (vertices.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr, // size of data in bytes
            vertices.as_ptr() as *const gl::types::GLvoid, // pointer to data
            gl::STATIC_DRAW, // usage
        );
        gl::BindBuffer(gl::ARRAY_BUFFER, 0); // unbind the buffer
    }

    let mut vao: gl::types::GLuint = 0;
    unsafe {
        gl::GenVertexArrays(1, &mut vao);
    }

    unsafe {
        gl::BindVertexArray(vao);
        
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(
            0, // index of the generic vertex attribute ("layout (location = 0)")
            3, // the number of components per generic vertex attribute
            gl::FLOAT, // data type
            gl::FALSE, // normalized (int-to-float conversion)
            (6 * std::mem::size_of::<f32>()) as gl::types::GLint, // stride (byte offset between consecutive attributes)
            std::ptr::null() // offset of the first component
        );
        gl::EnableVertexAttribArray(1);
        gl::VertexAttribPointer(
            1, // index of the generic vertex attribute ("layout (location = 0)")
            3, // the number of components per generic vertex attribute
            gl::FLOAT, // data type
            gl::FALSE, // normalized (int-to-float conversion)
            (6 * std::mem::size_of::<f32>()) as gl::types::GLint, // stride (byte offset between consecutive attributes)
            (3 * std::mem::size_of::<f32>()) as *const gl::types::GLvoid // offset of the first component
        );

        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindVertexArray(0);
    }

    // end code needed moving

    while !window.should_close() {
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            handle_window_event(&mut window, event);
        }
        draw(&[0; 5], &mut window, &shader_program, vao)
    }
}

fn draw(_board: &[i32], window: &mut glfw::Window, shader_program: &Program, vao: gl::types::GLuint) {
    //let _gl = gl::load_with(|s| window.get_proc_address(s) as *const std::os::raw::c_void);

    shader_program.set_used();
    unsafe {
        gl::BindVertexArray(vao);
        gl::DrawArrays(gl::TRIANGLES, 0, 3);
    }

    window.swap_buffers();
}

fn shader_from_source(source: &CStr, kind: gl::types::GLuint) -> Result<gl::types::GLuint, String> {
    let id = unsafe { gl::CreateShader(kind) };
    unsafe {
        gl::ShaderSource(id, 1, &source.as_ptr(), std::ptr::null());
        gl::CompileShader(id);
    }
    
    let mut success: gl::types::GLint = 1;
    unsafe {
        gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
    }

    if success == 0 {
        let mut len: gl::types::GLint = 0;
        unsafe {
            gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut len);
        }
        let error: CString = create_whitespace_cstring_with_len(len as usize);
        unsafe {
            gl::GetShaderInfoLog(
                id,
                len,
                std::ptr::null_mut(),
                error.as_ptr() as *mut gl::types::GLchar
            );
        }
        return Err(error.to_string_lossy().into_owned());
    }
    
    Ok(id)
}

fn create_whitespace_cstring_with_len(len: usize) -> CString {
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    buffer.extend([b' '].iter().cycle().take(len));
    unsafe { CString::from_vec_unchecked(buffer) }
}

fn _draw_checkers() {
}

fn handle_window_event(window: &mut glfw::Window, event: glfw::WindowEvent) {
    match event {
        glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
            window.set_should_close(true)
        }
        _ => {}
    }
}
