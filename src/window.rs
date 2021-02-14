extern crate glfw;

extern crate gl;
use gl::types::*;

extern crate image;
use image::GenericImage;

use std::sync::mpsc::Receiver;
use std::ffi::{CString, CStr};
use glfw::{Window, WindowEvent, Glfw, Context, Key, Action};

// following the tutorial from http://nercury.github.io/rust/opengl/tutorial/2018/02/10/opengl-in-rust-from-scratch-03-compiling-shaders.html

struct Shader {
    id: GLuint,
}

impl Shader {
    fn from_source(source: &CStr, kind: GLenum) -> Result<Shader, String> {
        let id = shader_from_source(source, kind)?;
        Ok(Shader { id })
    }

    fn from_vert_source(source: &CStr) -> Result<Shader, String> {
        Shader::from_source(source, gl::VERTEX_SHADER)
    }

    fn from_frag_source(source: &CStr) -> Result<Shader, String> {
        Shader::from_source(source, gl::FRAGMENT_SHADER)
    }

    fn id(&self) -> GLuint {
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
    id: GLuint,
}

impl Program {
    fn from_shaders(shaders: &[Shader]) -> Result<Program, String> {
        let program_id = unsafe { gl::CreateProgram() };

        for shader in shaders {
            unsafe { gl::AttachShader(program_id, shader.id()); }
        }

        unsafe { gl::LinkProgram(program_id); }

        let mut success: GLint = 1;
        unsafe {
            gl::GetProgramiv(program_id, gl::LINK_STATUS, &mut success);
        }

        if success == 0 {
            let mut len: GLint = 0;
            unsafe {
                gl::GetProgramiv(program_id, gl::INFO_LOG_LENGTH, &mut len);
            }

            let error = create_whitespace_cstring_with_len(len as usize);

            unsafe {
                gl::GetProgramInfoLog(
                    program_id,
                    len,
                    std::ptr::null_mut(),
                    error.as_ptr() as *mut GLchar
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

struct Texture {
    id: GLuint,
}

impl Texture {
    fn from_file(path: &str) -> Result<Program, String> {
        let mut texture_id = 0;
        gl::GenTextures(1, &mut texture_id);
        gl::BindTexture(gl::TEXTURE_2D, texture_id);

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

        let img = image::open(&std::path::Path::new(path)).expect("Failed to load texture");
        let data = img.raw_pixels();
        gl::TexImage2D(gl::TEXTURE_2D,
               0,
               gl::RGB as i32,
               img.width() as i32,
               img.height() as i32,
               0,
               gl::RGB,
               gl::UNSIGNED_BYTE,
               &data[0] as *const u8 as *const c_void);
        gl::GenerateMipmap(gl::TEXTURE_2D);
    }
}

pub struct Game {
    glfw: Glfw,
    window: Window,
    events: Receiver<(f64, WindowEvent)>,
    white_shader: Program,
    black_shader: Program,
    board: [GLuint; 64],
}

impl Game {
    pub fn new() -> Game {
        let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

        let (mut window, events) = glfw.create_window(800, 800, "Chess", glfw::WindowMode::Windowed)
            .expect("Failed to create GLFW window.");

        window.set_key_polling(true);
        window.make_current();

        window.get_proc_address("Chess");

        let _gl = gl::load_with(|s| window.get_proc_address(s) as *const std::os::raw::c_void);

        let (white_shader, black_shader) = Game::generate_shaders();
        let board = Game::generate_vaos();

        Game {
            glfw,
            window,
            events,
            white_shader,
            black_shader,
            board,
        }
    }

    pub fn game_loop(&mut self) {
        while !self.window.should_close() {
            self.handle_window_event();
            self.draw(&[0; 5]);
        }
    }
    
    fn handle_window_event(&mut self) {
        self.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&self.events) {
            match event {
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    self.window.set_should_close(true)
                }
                _ => {}
            }
        }
    }

    fn draw(&mut self,_board: &[i32]) {
        unsafe {
            gl::ClearColor(0.2, 0.3, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        self.draw_board();

        self.window.swap_buffers();
    }

    fn generate_shaders() -> (Program, Program) {
        let white_vert = Shader::from_vert_source(
            &CString::new(include_str!("white.vert")).unwrap()
        ).unwrap();

        let white_frag = Shader::from_frag_source(
            &CString::new(include_str!("white.frag")).unwrap()
        ).unwrap();

        let white_shaders = Program::from_shaders(
            &[white_vert, white_frag]
        ).unwrap();


        let black_vert = Shader::from_vert_source(
            &CString::new(include_str!("black.vert")).unwrap()
        ).unwrap();

        let black_frag = Shader::from_frag_source(
            &CString::new(include_str!("black.frag")).unwrap()
        ).unwrap();

        let black_shaders = Program::from_shaders(
            &[black_vert, black_frag]
        ).unwrap();

        (white_shaders, black_shaders)
    }

    fn generate_vaos() -> [GLuint; 64] {
        let generate_vao = |x: f32, y: f32| -> GLuint {
            let square_size: f32 = 2.0 / 8.0;
            let vertices: [f32; 12] = [
                x * square_size + square_size,    y * square_size + square_size,    0.0, // top right
                x * square_size + square_size,    y * square_size,                  0.0, // bottom right
                x * square_size,                  y * square_size,                  0.0, // bottom left
                x * square_size,                  y * square_size + square_size,    0.0, // top left
            ];

            let indices = [
                0, 1, 3,  // first Triangle
                1, 2, 3   // second Triangle
            ];

            let (mut vbo, mut vao, mut ebo) = (0, 0, 0);

            unsafe {
                gl::GenVertexArrays(1, &mut vao);
                gl::GenBuffers(1, &mut vbo);
                gl::GenBuffers(1, &mut ebo);

                gl::BindVertexArray(vao);

                gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
                gl::BufferData(
                    gl::ARRAY_BUFFER, // target
                    (vertices.len() * std::mem::size_of::<GLfloat>()) as GLsizeiptr, // size of data in bytes
                    &vertices[0] as *const f32 as *const GLvoid, // pointer to data
                    gl::STATIC_DRAW, // usage
                );

                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
                gl::BufferData(
                    gl::ELEMENT_ARRAY_BUFFER, // target
                    (indices.len() * std::mem::size_of::<GLfloat>()) as GLsizeiptr, // size of data in bytes
                    &indices[0] as *const i32 as *const GLvoid, // pointer to data
                    gl::STATIC_DRAW, // usage
                );

                let stride = 3 * std::mem::size_of::<GLfloat>() as GLsizei;

                gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, std::ptr::null());
                gl::EnableVertexAttribArray(0);

                gl::BindBuffer(gl::ARRAY_BUFFER, 0); // unbind the buffer
                gl::BindVertexArray(0);
            }

            vao
        };

        let mut voas: [GLuint; 64] = [0; 64]; 
        for i in 0..8 {
            for j in 0..8 {
                voas[i * 8 + j] = generate_vao(i as f32 - 4.0, j as f32 - 4.0);
            }
        }
        voas
    }

    fn draw_board(&self) {
        for i in 0..8 {
            for j in 0..8 {
                let square_color = if (i + j) % 2 == 0 { &self.black_shader } else { &self.white_shader };
                self.draw_square(square_color, self.board[i * 8 + j]);
            }
        }
    }
    
    fn draw_square(&self, program: &Program, vao: GLuint) {
        program.set_used();
        unsafe {
            gl::BindVertexArray(vao);
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, std::ptr::null());
        }
    }
}

fn shader_from_source(source: &CStr, kind: GLuint) -> Result<GLuint, String> {
    let id = unsafe { gl::CreateShader(kind) };
    unsafe {
        gl::ShaderSource(id, 1, &source.as_ptr(), std::ptr::null());
        gl::CompileShader(id);
    }
    
    let mut success: GLint = 1;
    unsafe {
        gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
    }

    if success == 0 {
        let mut len: GLint = 0;
        unsafe {
            gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut len);
        }
        let error: CString = create_whitespace_cstring_with_len(len as usize);
        unsafe {
            gl::GetShaderInfoLog(
                id,
                len,
                std::ptr::null_mut(),
                error.as_ptr() as *mut GLchar
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
