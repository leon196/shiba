mod settings;

pub use self::settings::Settings;
use crate::configuration::Configuration;
use crate::custom_codes::CustomCodes;
use crate::generator_utils::cpp;
use crate::paths::TEMP_DIRECTORY;
use crate::traits;
use crate::types::{CompilationDescriptor, Pass, ShaderDescriptor, UniformArray};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tera::Tera;

#[derive(Serialize)]
struct Context<'a> {
	api: &'a String,
	custom_codes: &'a CustomCodes,
	opengl_declarations: &'a String,
	opengl_loading: &'a String,
	passes: &'a [Pass],
	render: &'a String,
	shader_declarations: &'a String,
	shader_loading: &'a String,
	uniform_arrays: &'a [UniformArray],
}

pub struct Generator<'a> {
	cpp_template_renderer: cpp::TemplateRenderer,
	glew_path: PathBuf,
	msvc_command_generator: cpp::msvc::CommandGenerator,
	settings: &'a Settings,
	tera: Tera,
}

impl<'a> Generator<'a> {
	pub fn new(settings: &'a Settings, configuration: &'a Configuration) -> Result<Self, String> {
		let cpp_template_renderer = cpp::TemplateRenderer::new(configuration)?;
		let glew_path = configuration
			.paths
			.get("glew")
			.ok_or("Please set configuration key paths.glew.")?
			.clone();
		let msvc_command_generator = cpp::msvc::CommandGenerator::new()?;

		let mut tera = Tera::default();

		tera.add_raw_template("template", include_str!("template.tera"))
			.map_err(|err| err.to_string())?;

		Ok(Generator {
			cpp_template_renderer,
			glew_path,
			msvc_command_generator,
			settings,
			tera,
		})
	}

	pub fn get_path() -> PathBuf {
		TEMP_DIRECTORY.join("blender_api.dll")
	}
}

impl<'a> traits::Generator for Generator<'a> {
	fn generate(
		&self,
		compilation_descriptor: &CompilationDescriptor,
		custom_codes: &CustomCodes,
		shader_descriptor: &ShaderDescriptor,
	) -> Result<(), String> {
		let contents = self.cpp_template_renderer.render(
			custom_codes,
			shader_descriptor,
			true,
			"blender_api",
		)?;

		let context = Context {
			api: &contents.api,
			custom_codes: &custom_codes,
			opengl_declarations: &contents.opengl_declarations,
			opengl_loading: &contents.opengl_loading,
			passes: &shader_descriptor.passes,
			render: &contents.render,
			shader_declarations: &contents.shader_declarations,
			shader_loading: &contents.shader_loading,
			uniform_arrays: &shader_descriptor.uniform_arrays,
		};
		let contents = self
			.tera
			.render("template", &context)
			.map_err(|_| "Failed to render template.")?;

		fs::write(TEMP_DIRECTORY.join("blender_api.cpp"), contents.as_bytes())
			.map_err(|_| "Failed to write to file.")?;

		let mut compilation = self
			.msvc_command_generator
			.command(cpp::msvc::Platform::X64)
			.arg("cl")
			.arg("/c")
			.arg("/EHsc")
			.arg("/FA")
			.arg("/Fablender_api.asm")
			.arg("/Foblender_api.obj")
			.arg(format!(
				"/I{}",
				self.glew_path.join("include").to_string_lossy(),
			))
			.args(&compilation_descriptor.cl.args)
			.arg("blender_api.cpp")
			.arg("&&")
			.arg("link")
			.arg("/DLL")
			.arg("/OUT:blender_api.dll")
			.args(&self.settings.link.args)
			.arg(
				self.glew_path
					.join("lib")
					.join("Release")
					.join("x64")
					.join("glew32s.lib")
					.to_string_lossy()
					.as_ref(),
			)
			.args(&compilation_descriptor.link.args)
			.arg("blender_api.obj")
			.current_dir(&*TEMP_DIRECTORY)
			.spawn()
			.map_err(|err| err.to_string())?;

		let status = compilation.wait().map_err(|err| err.to_string())?;
		if !status.success() {
			return Err("Failed to compile.".to_string());
		}

		Ok(())
	}

	fn get_path(&self) -> PathBuf {
		Generator::get_path()
	}
}
