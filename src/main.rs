use std::{sync::Arc, thread, time::Duration};

use ash::vk::{self, ApplicationInfo, DeviceQueueCreateInfo};
use openxr::{
    CompositionLayerFlags, CompositionLayerProjection, CompositionLayerProjectionView,
    EnvironmentBlendMode, ExtensionSet, FormFactor, ReferenceSpaceType, SessionState,
    SwapchainCreateFlags, SwapchainSubImage, SwapchainUsageFlags, ViewConfigurationType, Vulkan,
};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use vulkano::{
    DeviceSize, Handle, Version, VulkanObject,
    buffer::{BufferCreateInfo, BufferUsage},
    command_buffer::{
        CommandBufferUsage, CopyBufferToImageInfo, PrimaryCommandBufferAbstract,
        allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo},
    },
    device::QueueCreateInfo,
    format::Format,
    image::{ImageUsage, sys::RawImage},
    memory::{MemoryPropertyFlags, allocator::AllocationCreateInfo},
    sync::GpuFuture,
};

// const COLOR: [u8;4] = [255, 0, 0, 25u8];
// const COLOR: [u8; 4] = [236, 133, 161, 40u8];
// const COLOR: [u8; 4] = [148, 80, 99, 40u8];
const COLOR: [u8; 4] = [0, 0, 0, 220u8];
const PREMUL: bool = false;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    info!("Hewow oworld");
    let oxr_entry = unsafe { openxr::Entry::load() }.unwrap();
    let vk_entry = unsafe { ash::Entry::load() }.unwrap();
    let vk_get_instance_proc_addr = vk_entry.static_fn().get_instance_proc_addr;
    let vk_entry =
        vulkano::library::VulkanLibrary::with_loader(AshEntryVulkanoLoader(vk_entry)).unwrap();
    let exts = oxr_entry.enumerate_extensions().unwrap();
    let mut enabled_exts = ExtensionSet::default();
    if !exts.khr_vulkan_enable2 {
        panic!("no vulkan");
    }
    enabled_exts.khr_vulkan_enable2 = true;
    let oxr_instance = oxr_entry
        .create_instance(
            &openxr::ApplicationInfo {
                application_name: "UwU",
                application_version: 1,
                engine_name: "Nya",
                engine_version: 1,
                api_version: openxr::Version::new(1, 1, 0),
            },
            &enabled_exts,
            &[],
        )
        .unwrap();
    let system = oxr_instance
        .system(FormFactor::HEAD_MOUNTED_DISPLAY)
        .unwrap();
    let system_props = oxr_instance.system_properties(system).unwrap();
    info!(?system_props);
    let vk_instance = unsafe {
        oxr_instance.create_vulkan_instance(
            system,
            #[expect(clippy::missing_transmute_annotations)]
            std::mem::transmute(vk_get_instance_proc_addr),
            &ash::vk::InstanceCreateInfo::default().application_info(
                &ApplicationInfo::default()
                    .api_version(ash::vk::API_VERSION_1_3)
                    .application_version(1)
                    .application_name(c"UwU")
                    .engine_name(c"Nya")
                    .engine_version(1),
            ) as *const _ as *const _,
        )
    }
    .unwrap()
    .unwrap();
    let vk_instance = unsafe {
        vulkano::instance::Instance::from_handle(
            vk_entry,
            ash::vk::Instance::from_raw(vk_instance as u64),
            vulkano::instance::InstanceCreateInfo {
                application_name: Some("UwU".into()),
                application_version: Version::V1_0,
                engine_name: Some("Nya".into()),
                engine_version: Version::V1_0,
                max_api_version: Some(Version::V1_3),
                ..Default::default()
            },
        )
    };
    let vk_phys_dev =
        unsafe { oxr_instance.vulkan_graphics_device(system, vk_instance.handle().as_raw() as _) }
            .unwrap();
    let vk_phys_dev = unsafe {
        vulkano::device::physical::PhysicalDevice::from_handle(
            vk_instance.clone(),
            ash::vk::PhysicalDevice::from_raw(vk_phys_dev as u64),
        )
        .unwrap()
    };
    let vk_dev = ash::vk::Device::from_raw(
        unsafe {
            oxr_instance.create_vulkan_device(
                system,
                #[expect(clippy::missing_transmute_annotations)]
                std::mem::transmute(vk_get_instance_proc_addr),
                vk_phys_dev.handle().as_raw() as _,
                &vk::DeviceCreateInfo::default().queue_create_infos(&[DeviceQueueCreateInfo {
                    // very bad, but works out for bevy_mod_openxr, somehow
                    queue_family_index: 0,
                    ..Default::default()
                }
                .queue_priorities(&[1.0])]) as *const _ as _,
            )
        }
        .unwrap()
        .unwrap() as u64,
    );
    let (vk_dev, mut queues) = unsafe {
        vulkano::device::Device::from_handle(
            vk_phys_dev.clone(),
            vk_dev,
            vulkano::device::DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index: 0,
                    queues: vec![1.0],
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
    };
    let queue = queues.next().unwrap();
    let _props = oxr_instance
        .graphics_requirements::<Vulkan>(system)
        .unwrap();
    let (oxr_session, mut oxr_frame_waiter, mut oxr_frame_stream) = unsafe {
        oxr_instance.create_session::<Vulkan>(
            system,
            &openxr::vulkan::SessionCreateInfo {
                instance: vk_instance.handle().as_raw() as _,
                physical_device: vk_phys_dev.handle().as_raw() as _,
                device: vk_dev.handle().as_raw() as _,
                queue_family_index: queue.queue_family_index(),
                queue_index: queue.queue_index(),
            },
        )
    }
    .unwrap();

    let res = oxr_instance
        .enumerate_view_configuration_views(system, ViewConfigurationType::PRIMARY_STEREO)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    let mut swapchain = oxr_session
        .create_swapchain(&openxr::SwapchainCreateInfo {
            create_flags: SwapchainCreateFlags::EMPTY,
            usage_flags: SwapchainUsageFlags::TRANSFER_DST | SwapchainUsageFlags::COLOR_ATTACHMENT,
            format: ash::vk::Format::R8G8B8A8_UNORM.as_raw() as u32,
            sample_count: 1,
            width: res.recommended_image_rect_width,
            height: res.recommended_image_rect_height,
            face_count: 1,
            array_size: 2,
            mip_count: 1,
        })
        .unwrap();
    let command_alloc = Arc::new(StandardCommandBufferAllocator::new(
        vk_dev.clone(),
        StandardCommandBufferAllocatorCreateInfo {
            primary_buffer_count: 1,
            ..Default::default()
        },
    ));
    let alloc =
        Arc::new(vulkano::memory::allocator::StandardMemoryAllocator::new_default(vk_dev.clone()));
    let data_size = res.recommended_image_rect_width * res.recommended_image_rect_height * 4 * 2;
    let images = swapchain
        .enumerate_images()
        .unwrap()
        .into_iter()
        .map(|i| unsafe {
            RawImage::from_handle_borrowed(
                vk_dev.clone(),
                ash::vk::Image::from_raw(i),
                vulkano::image::ImageCreateInfo {
                    image_type: vulkano::image::ImageType::Dim2d,
                    format: Format::R8G8B8A8_UNORM,
                    extent: [
                        res.recommended_image_rect_width,
                        res.recommended_image_rect_height,
                        1,
                    ],
                    array_layers: 2,
                    mip_levels: 1,
                    samples: vulkano::image::SampleCount::Sample1,
                    usage: ImageUsage::TRANSFER_DST,
                    initial_layout: vulkano::image::ImageLayout::General,
                    ..Default::default()
                },
            )
            .unwrap()
            .assume_bound()
        })
        .map(Arc::new)
        .collect::<Vec<_>>();
    let ref_space = oxr_session
        .create_reference_space(ReferenceSpaceType::LOCAL, openxr::Posef::IDENTITY)
        .unwrap();
    let mut session_running = false;
    info!("create buffer");
    let buffer = vulkano::buffer::Buffer::new_slice::<u8>(
        alloc.clone(),
        BufferCreateInfo {
            size: 0,
            usage: BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: vulkano::memory::allocator::MemoryTypeFilter {
                required_flags: MemoryPropertyFlags::HOST_VISIBLE
                    | MemoryPropertyFlags::HOST_COHERENT,
                ..Default::default()
            },
            ..Default::default()
        },
        data_size as DeviceSize,
    )
    .unwrap();
    info!("mapping buffer");
    let mut mapped_buffer = buffer.write().unwrap();
    COLOR
        .into_iter()
        .cycle()
        .take(data_size as usize)
        .enumerate()
        .for_each(|(i, v)| mapped_buffer[i] = v);
    drop(mapped_buffer);
    let mut i = 0;
    loop {
        let mut event_buffer = openxr::EventDataBuffer::new();
        while let Some(e) = oxr_instance.poll_event(&mut event_buffer).unwrap() {
            match e {
                openxr::Event::EventsLost(_events_lost) => todo!(),
                openxr::Event::InstanceLossPending(_instance_loss_pending) => todo!(),
                openxr::Event::SessionStateChanged(session_state_changed) => {
                    match dbg!(session_state_changed.state()) {
                        SessionState::READY => {
                            oxr_session
                                .begin(ViewConfigurationType::PRIMARY_STEREO)
                                .unwrap();
                            session_running = true;
                        }
                        SessionState::STOPPING
                        | SessionState::LOSS_PENDING
                        | SessionState::IDLE
                        | SessionState::EXITING => session_running = false,
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        if !session_running {
            thread::sleep(Duration::from_millis(5));
            continue;
        }
        info!("waiting");
        let state = oxr_frame_waiter.wait().unwrap();
        info!("waited");
        oxr_frame_stream.begin().unwrap();

        if i < 16 {
            let image = swapchain.acquire_image().unwrap();
            info!("waiting image");
            swapchain.wait_image(openxr::Duration::INFINITE).unwrap();

            let image = images[image as usize].clone();
            info!("building cmd buffer");
            let mut cmd = vulkano::command_buffer::AutoCommandBufferBuilder::primary(
                command_alloc.clone(),
                queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap();
            cmd.copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(buffer.clone(), image))
                .unwrap();
            info!("awaiting cmd buffer");
            cmd.build()
                .unwrap()
                .execute(queue.clone())
                .unwrap()
                .then_signal_fence_and_flush()
                .unwrap()
                .wait(None)
                .unwrap();
            info!("releasing image");
            swapchain.release_image().unwrap();
            i += 1;
        }

        let (_, views) = oxr_session
            .locate_views(
                ViewConfigurationType::PRIMARY_STEREO,
                state.predicted_display_time,
                &ref_space,
            )
            .unwrap();
        // if views.iter().any(|v| {
        //     let q = v.pose.orientation;
        //     (q.x + q.y + q.z + q.w) < f32::EPSILON
        // }) {
        //     continue;
        // }

        _ = oxr_frame_stream
            .end(
                state.predicted_display_time,
                EnvironmentBlendMode::ALPHA_BLEND,
                &[&CompositionLayerProjection::new()
                    .space(&ref_space)
                    .layer_flags(match PREMUL {
                        true => CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA,
                        false => {
                            CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA
                                | CompositionLayerFlags::UNPREMULTIPLIED_ALPHA
                        }
                    })
                    .views(&[
                        CompositionLayerProjectionView::new()
                            .fov(views[0].fov)
                            .pose(views[0].pose)
                            .sub_image(
                                SwapchainSubImage::new()
                                    .swapchain(&swapchain)
                                    .image_array_index(0)
                                    .image_rect(openxr::Rect2Di {
                                        offset: openxr::Offset2Di { x: 0, y: 0 },
                                        extent: openxr::Extent2Di {
                                            width: res.recommended_image_rect_width as i32,
                                            height: res.recommended_image_rect_height as i32,
                                        },
                                    }),
                            ),
                        CompositionLayerProjectionView::new()
                            .fov(views[1].fov)
                            .pose(views[1].pose)
                            .sub_image(
                                SwapchainSubImage::new()
                                    .swapchain(&swapchain)
                                    .image_array_index(1)
                                    .image_rect(openxr::Rect2Di {
                                        offset: openxr::Offset2Di { x: 0, y: 0 },
                                        extent: openxr::Extent2Di {
                                            width: res.recommended_image_rect_width as i32,
                                            height: res.recommended_image_rect_height as i32,
                                        },
                                    }),
                            ),
                    ])],
            )
            .inspect_err(|err| error!("{err}"));
        info!("frame done");
    }
}

struct AshEntryVulkanoLoader(ash::Entry);
unsafe impl vulkano::library::Loader for AshEntryVulkanoLoader {
    unsafe fn get_instance_proc_addr(
        &self,
        instance: ash::vk::Instance,
        name: *const std::os::raw::c_char,
    ) -> ash::vk::PFN_vkVoidFunction {
        unsafe { self.0.get_instance_proc_addr(instance, name) }
    }
}
