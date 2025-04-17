use nalgebra::Vector3;

type Vector3f = Vector3<f32>;

#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera {
    /// matrix parameters
    up_vector: [f32; 3],
    _pad0: u32,
    position: [f32; 3],
    _pad1: u32,
    look_at: [f32; 3],
    _pad2: u32,

    /// basis,
    // NOTE: draw happens top-left to right-bottom
    // +Z
    forward: [f32; 3],
    _pad3: u32,
    // draw toward -X
    right: [f32; 3],
    _pad4: u32,
    // draw toward -Y
    up: [f32; 3],
    _pad5: u32,

    /// photo parameters
    // in mm
    focal_length: f32,
    // in m
    focus_distance: f32,
    // aperture denominator
    aperture: f32,
    // in mm
    sensor_height: f32,
    aspect_ratio: f32,
    picture_width: u32,

    ///derived
    // in mm
    pub aperture_radius: f32,
    // vertical angle
    pub fovy: f32,

    // in mm
    // minimum aperture radius, the sharpest picture
    pub min_coc: f32,
    _pad6: [u32; 3],
}

const _: () = assert!(std::mem::size_of::<Camera>() % 16 == 0);
// const _: () = assert!(std::mem::align_of::<Camera>() == 16);

impl Default for Camera {
    fn default() -> Self {
        let min_coc = 2.0;
        let up_vector: [f32; 3] = Vector3f::y().into();
        let position: [f32; 3] = Vector3f::zeros().into();
        let look_at: [f32; 3] = Vector3f::z().into();

        // camera picture spec
        let sensor_height = 24.0;
        let aspect_ratio = 16.0 / 9.0;

        let picture_width = 1920;

        let focal_length = 50.0;
        let focus_distance = 1.0;

        let aperture = 2.8;

        let forward = look_at;
        let right: [f32; 3] = Vector3f::x().into();
        let up = up_vector;

        let mut camera = Self {
            up_vector,
            _pad0: 0,
            position,
            _pad1: 0,
            look_at,
            _pad2: 0,

            _pad3: 0,
            forward,
            _pad4: 0,
            right,
            _pad5: 0,
            up,

            focal_length,
            focus_distance,
            aperture,
            sensor_height,
            aspect_ratio,
            picture_width,
            aperture_radius: 0.0,
            fovy: 0.0,
            min_coc,
            _pad6: [0; 3],
        };

        camera.update_camera_config();
        camera
    }
}

impl Camera {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_aperture(&mut self, aperture: f32) {
        self.aperture = aperture;
        self.update_camera_config();
    }

    pub fn set_resolution(&mut self, width: u32, height: u32, compensate_fov: bool) {
        self.aspect_ratio = width as f32 / height as f32;
        self.picture_width = width;
        if compensate_fov {
            let d: f32 = self.fovy * 0.5;
            let d = 2.0 * d.tan();
            self.focal_length = self.sensor_height / d;
        }
        self.update_camera_config();
    }

    pub fn set_focal_length(&mut self, focal_length: f32) {
        self.focal_length = focal_length;
        self.update_camera_config();
    }

    fn update_camera_config(&mut self) {
        let sensor_width: f32 = self.sensor_height * self.aspect_ratio;
        // TODO: add max aperture radius as a member variable
        let max_aperture_radius = self.sensor_height.min(sensor_width) / 2.0;

        let aperture_radius: f32 = self.focal_length / (2.0 * self.aperture);
        self.aperture_radius = aperture_radius.min(max_aperture_radius).max(self.min_coc);

        let fovy: f32 = self.sensor_height / (2.0 * self.focal_length);
        self.fovy = 2.0 * fovy.atan();
    }
}
