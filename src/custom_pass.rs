use std::path::PathBuf;

use amethyst::{
    core::ecs::{
        Component, DenseVecStorage, DispatcherBuilder, Join, ReadStorage, SystemData, World,
    },
    prelude::*,
    renderer::{
        bundle::{RenderOrder, RenderPlan, RenderPlugin, Target},
        pipeline::{PipelineDescBuilder, PipelinesBuilder},
        rendy::{
            command::{QueueId, RenderPassEncoder},
            factory::Factory,
            graph::{
                render::{PrepareResult, RenderGroup, RenderGroupDesc},
                GraphContext, NodeBuffer, NodeImage,
            },
            hal::{self, device::Device, format::Format, pso},
            mesh::{AsVertex, VertexFormat},
            shader::{PathBufShaderInfo, Shader, ShaderKind, SourceLanguage, SpirvShader},
        },
        submodules::{DynamicUniform, DynamicVertexBuffer},
        types::Backend,
        util, ChangeDetection,
    },
    window::ScreenDimensions,
};

use amethyst_error::Error;
use derivative::Derivative;
use glsl_layout::*;

lazy_static::lazy_static! {
    static ref VERTEX: SpirvShader = PathBufShaderInfo::new(
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/custom.vert")),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main"
    ).precompile().unwrap();

    static ref FRAGMENT: SpirvShader = PathBufShaderInfo::new(
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/custom.frag")),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main"
    ).precompile().unwrap();
}

#[derive(Clone, Debug, PartialEq, Derivative)]
#[derivative(Default(bound = ""))]
pub struct DrawCustomDesc;

impl DrawCustomDesc {
    pub fn new() -> Self {
        Default::default()
    }
}

impl<B: Backend> RenderGroupDesc<B, World> for DrawCustomDesc {
    fn build(
        self,
        _ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        _queue: QueueId,
        _world: &World,
        framebuffer_width: u32,
        framebuffer_height: u32,
        subpass: hal::pass::Subpass<'_, B>,
        _buffers: Vec<NodeBuffer>,
        _images: Vec<NodeImage>,
    ) -> Result<Box<dyn RenderGroup<B, World>>, failure::Error> {
        let env = DynamicUniform::new(factory, pso::ShaderStageFlags::VERTEX)?;
        let vertex = DynamicVertexBuffer::new();

        let (pipeline, pipeline_layout) = build_custom_pipeline(
            factory,
            subpass,
            framebuffer_width,
            framebuffer_height,
            vec![env.raw_layout()],
        )?;

        Ok(Box::new(DrawCustom::<B> {
            pipeline,
            pipeline_layout,
            env,
            vertex,
            vertex_count: 0,
            change: Default::default(),
        }))
    }
}

#[derive(Debug)]
pub struct DrawCustom<B: Backend> {
    pipeline: B::GraphicsPipeline,
    pipeline_layout: B::PipelineLayout,
    env: DynamicUniform<B, CustomUniformArgs>,
    vertex: DynamicVertexBuffer<B, CustomArgs>,
    vertex_count: usize,
    change: ChangeDetection,
}

impl<B: Backend> RenderGroup<B, World> for DrawCustom<B> {
    fn prepare(
        &mut self,
        factory: &Factory<B>,
        _queue: QueueId,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        world: &World,
    ) -> PrepareResult {
        let (quads,) = <(ReadStorage<'_, Quad>,)>::fetch(world);

        let args = world.read_resource::<CustomUniformArgs>();

        self.env.write(factory, index, args.std140());

        let old_vertex_count = self.vertex_count;

        self.vertex_count = quads.join().count() * 4;

        let changed = old_vertex_count != self.vertex_count;

        let vertex_data_iter = quads.join().flat_map(|triangle| triangle.get_args());

        self.vertex.write(
            factory,
            index,
            self.vertex_count as u64,
            Some(vertex_data_iter.collect::<Box<[CustomArgs]>>()),
        );
        self.change.prepare_result(index, changed)
    }

    fn draw_inline(
        &mut self,
        mut encoder: RenderPassEncoder<'_, B>,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        _world: &World,
    ) {
        if self.vertex_count == 0 {
            return;
        }

        encoder.bind_graphics_pipeline(&self.pipeline);

        self.env.bind(index, &self.pipeline_layout, 0, &mut encoder);

        self.vertex.bind(index, 0, 0, &mut encoder);

        unsafe {
            encoder.draw(0..self.vertex_count as u32, 0..1);
        }
    }

    fn dispose(self: Box<Self>, factory: &mut Factory<B>, _world: &World) {
        unsafe {
            factory.device().destroy_graphics_pipeline(self.pipeline);
            factory
                .device()
                .destroy_pipeline_layout(self.pipeline_layout);
        }
    }
}

fn build_custom_pipeline<B: Backend>(
    factory: &Factory<B>,
    subpass: hal::pass::Subpass<'_, B>,
    framebuffer_width: u32,
    framebuffer_height: u32,
    layouts: Vec<&B::DescriptorSetLayout>,
) -> Result<(B::GraphicsPipeline, B::PipelineLayout), failure::Error> {
    let pipeline_layout = unsafe {
        factory
            .device()
            .create_pipeline_layout(layouts, None as Option<(_, _)>)
    }?;

    let shader_vertex = unsafe { VERTEX.module(factory).unwrap() };
    let shader_fragment = unsafe { FRAGMENT.module(factory).unwrap() };

    let pipes = PipelinesBuilder::new()
        .with_pipeline(
            PipelineDescBuilder::new()
                .with_vertex_desc(&[(CustomArgs::vertex(), pso::VertexInputRate::Vertex)])
                .with_input_assembler(pso::InputAssemblerDesc::new(hal::Primitive::TriangleStrip))
                .with_shaders(util::simple_shader_set(
                    &shader_vertex,
                    Some(&shader_fragment),
                ))
                .with_layout(&pipeline_layout)
                .with_subpass(subpass)
                .with_framebuffer_size(framebuffer_width, framebuffer_height)
                .with_blend_targets(vec![pso::ColorBlendDesc {
                    mask: pso::ColorMask::ALL,
                    blend: Some(pso::BlendState::ALPHA),
                }]),
        )
        .build(factory, None);

    unsafe {
        factory.destroy_shader_module(shader_vertex);
        factory.destroy_shader_module(shader_fragment);
    }

    match pipes {
        Err(e) => {
            unsafe {
                factory.device().destroy_pipeline_layout(pipeline_layout);
            }
            Err(e)
        }
        Ok(mut pipes) => Ok((pipes.remove(0), pipeline_layout)),
    }
}

#[derive(Default, Debug)]
pub struct RenderCustom {}

impl<B: Backend> RenderPlugin<B> for RenderCustom {
    fn on_build<'a, 'b>(
        &mut self,
        world: &mut World,
        _builder: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        let aspect_ratio = world.read_resource::<ScreenDimensions>().aspect_ratio();
        world.register::<Quad>();
        world.insert(CustomUniformArgs {
            scale: 1.0,
            offset: [0.0; 2].into(),
            aspect_ratio,
            max_iters: 100,
        });
        Ok(())
    }

    fn on_plan(
        &mut self,
        plan: &mut RenderPlan<B>,
        _factory: &mut Factory<B>,
        _world: &World,
    ) -> Result<(), Error> {
        plan.extend_target(Target::Main, |ctx| {
            ctx.add(RenderOrder::Transparent, DrawCustomDesc::new().builder())?;
            Ok(())
        });
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, AsStd140)]
#[repr(C, align(4))]
pub struct CustomArgs {
    pub pos: vec2,
}

impl AsVertex for CustomArgs {
    fn vertex() -> VertexFormat {
        VertexFormat::new(((Format::Rg32Sfloat, "pos"),))
    }
}

#[derive(Clone, Copy, Debug, AsStd140)]
#[repr(C, align(4))]
pub struct CustomUniformArgs {
    pub scale: float,
    pub offset: vec2,
    pub aspect_ratio: f32,
    pub max_iters: i32,
}

#[derive(Debug, Default)]
pub struct Quad {
    pub points: [[f32; 2]; 4],
}

impl Component for Quad {
    type Storage = DenseVecStorage<Self>;
}

impl Quad {
    pub fn get_args(&self) -> Vec<CustomArgs> {
        let mut vec = Vec::new();
        vec.extend((0..4).map(|i| CustomArgs {
            pos: self.points[i].into(),
        }));
        vec
    }
}
