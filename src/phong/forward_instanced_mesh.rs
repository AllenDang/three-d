
use crate::core::*;
use crate::objects::*;
use crate::lights::*;
use crate::phong::*;

pub struct PhongForwardInstancedMesh {
    gl: Context,
    pub name: String,
    mesh: InstancedMesh,
    pub material: PhongMaterial
}

impl PhongForwardInstancedMesh
{
    pub fn new(gl: &Context, transformations: &[Mat4], cpu_mesh: &CPUMesh, material: &PhongMaterial) -> Result<Self, Error>
    {
        if cpu_mesh.normals.is_none() {
            Err(Error::FailedToCreateMesh {message:
              "Cannot create a mesh without normals. Consider calling compute_normals on the CPUMesh before creating the mesh.".to_string()})?
        }
        let mesh = InstancedMesh::new(gl, transformations, cpu_mesh)?;
        unsafe {
            INSTANCED_MESH_COUNT += 1;
        }
        Ok(Self {
            gl: gl.clone(),
            name: cpu_mesh.name.clone(),
            mesh,
            material: material.clone()
        })
    }

    pub fn update_transformations(&mut self, transformations: &[Mat4])
    {
        self.mesh.update_transformations(transformations);
    }

    pub fn render_depth(&self, render_states: RenderStates, viewport: Viewport, transformation: &Mat4, camera: &camera::Camera) -> Result<(), Error>
    {
        self.render_with_ambient(render_states, viewport, transformation, camera, &AmbientLight::default())
    }

    pub fn render_with_ambient(&self, render_states: RenderStates, viewport: Viewport, transformation: &Mat4, camera: &camera::Camera, ambient_light: &AmbientLight) -> Result<(), Error>
    {
        let program = match self.material.color_source {
            ColorSource::Color(_) => {
                unsafe {
                    if INSTANCED_PROGRAM_COLOR_AMBIENT.is_none()
                    {
                        INSTANCED_PROGRAM_COLOR_AMBIENT = Some(InstancedMesh::create_program(&self.gl, include_str!("shaders/colored_forward_ambient.frag"))?);
                    }
                    INSTANCED_PROGRAM_COLOR_AMBIENT.as_ref().unwrap()
                }
            },
            ColorSource::Texture(_) => {
                unsafe {
                    if INSTANCED_PROGRAM_TEXTURE_AMBIENT.is_none()
                    {
                        INSTANCED_PROGRAM_TEXTURE_AMBIENT = Some(InstancedMesh::create_program(&self.gl, include_str!("shaders/textured_forward_ambient.frag"))?);
                    }
                    INSTANCED_PROGRAM_TEXTURE_AMBIENT.as_ref().unwrap()
                }
            }
        };
        program.add_uniform_vec3("ambientColor", &(ambient_light.color * ambient_light.intensity))?;

        match self.material.color_source {
            ColorSource::Color(ref color) => {
                program.add_uniform_vec4("surfaceColor", color)?;
            },
            ColorSource::Texture(ref texture) => {
                if !self.mesh.has_uvs() {
                    Err(Error::FailedToCreateMesh {message:"Cannot use a texture as color source without uv coordinates.".to_string()})?;
                }
                program.use_texture(texture.as_ref(),"tex")?;
            }
        }
        self.mesh.render(program, render_states, viewport, transformation, camera)
    }

    pub fn render_with_ambient_and_directional(&self, render_states: RenderStates, viewport: Viewport, transformation: &Mat4, camera: &camera::Camera, ambient_light: &AmbientLight, directional_light: &DirectionalLight) -> Result<(), Error>
    {
        let program = match self.material.color_source {
            ColorSource::Color(_) => {
                unsafe {
                    if INSTANCED_PROGRAM_COLOR_AMBIENT_DIRECTIONAL.is_none()
                    {
                        INSTANCED_PROGRAM_COLOR_AMBIENT_DIRECTIONAL = Some(InstancedMesh::create_program(&self.gl, &format!("{}\n{}",
                                                                                      &include_str!("shaders/light_shared.frag"),
                                                                                      &include_str!("shaders/colored_forward_ambient_directional.frag")))?);
                    }
                    INSTANCED_PROGRAM_COLOR_AMBIENT_DIRECTIONAL.as_ref().unwrap()
                }
            },
            ColorSource::Texture(_) => {
                unsafe {
                    if INSTANCED_PROGRAM_TEXTURE_AMBIENT_DIRECTIONAL.is_none()
                    {
                        INSTANCED_PROGRAM_TEXTURE_AMBIENT_DIRECTIONAL = Some(InstancedMesh::create_program(&self.gl, &format!("{}\n{}",
                                                                                    include_str!("shaders/light_shared.frag"),
                                                                                    include_str!("shaders/textured_forward_ambient_directional.frag")))?)
                    }
                    INSTANCED_PROGRAM_TEXTURE_AMBIENT_DIRECTIONAL.as_ref().unwrap()
                }
            }
        };
        program.add_uniform_vec3("ambientColor", &(ambient_light.color * ambient_light.intensity))?;
        program.add_uniform_vec3("eyePosition", &camera.position())?;
        program.use_texture(directional_light.shadow_map(), "shadowMap")?;
        program.use_uniform_block(directional_light.buffer(), "DirectionalLightUniform");

        program.add_uniform_float("diffuse_intensity", &self.material.diffuse_intensity)?;
        program.add_uniform_float("specular_intensity", &self.material.specular_intensity)?;
        program.add_uniform_float("specular_power", &self.material.specular_power)?;

        match self.material.color_source {
            ColorSource::Color(ref color) => {
                program.add_uniform_vec4("surfaceColor", color)?;
            },
            ColorSource::Texture(ref texture) => {
                if !self.mesh.has_uvs() {
                    Err(Error::FailedToCreateMesh {message:"Cannot use a texture as color source without uv coordinates.".to_string()})?;
                }
                program.use_texture(texture.as_ref(),"tex")?;
            }
        }
        self.mesh.render(program, render_states, viewport, transformation, camera)
    }
}

impl Drop for PhongForwardInstancedMesh {

    fn drop(&mut self) {
        unsafe {
            INSTANCED_MESH_COUNT -= 1;
            if INSTANCED_MESH_COUNT == 0 {
                INSTANCED_PROGRAM_COLOR_AMBIENT = None;
                INSTANCED_PROGRAM_COLOR_AMBIENT_DIRECTIONAL = None;
                INSTANCED_PROGRAM_TEXTURE_AMBIENT = None;
                INSTANCED_PROGRAM_TEXTURE_AMBIENT_DIRECTIONAL = None;
            }
        }
    }
}

static mut INSTANCED_PROGRAM_COLOR_AMBIENT: Option<Program> = None;
static mut INSTANCED_PROGRAM_COLOR_AMBIENT_DIRECTIONAL: Option<Program> = None;
static mut INSTANCED_PROGRAM_TEXTURE_AMBIENT: Option<Program> = None;
static mut INSTANCED_PROGRAM_TEXTURE_AMBIENT_DIRECTIONAL: Option<Program> = None;
static mut INSTANCED_MESH_COUNT: u32 = 0;