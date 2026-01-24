use nalgebra::Vector3;

type Vector3f = Vector3<f32>;

#[derive(Copy, Clone, Debug)]
pub struct Camera {
    /// matrix parameters
    pub up_vector: [f32; 3],
    pub position: [f32; 3],
    pub look_at: [f32; 3],

    /// basis,
    // NOTE: draw happens top-left to right-bottom
    // +Z
    forward: [f32; 3],
    // draw toward -X
    right: [f32; 3],
    // draw toward -Y
    up: [f32; 3],

    /// photo parameters
    // in mm
    focal_length: f32,
    // in meters
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
    pub yaw: f32,
    pub pitch: f32,
    pub sensitivity: f32,
}


#[repr(C, packed)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraLean {
    pixeloo: [f32; 3],
    _pad0: u32,
    pixel_delta_u: [f32; 3],
    _pad1: u32,
    pixel_delta_v: [f32; 3],
    _pad2: u32,
    pos: [f32; 3],
    _pad3: u32,
}


#[repr(C, packed)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraBasis {
    right: [f32; 3],
    _pad0: u32,
    up: [f32; 3],
    _pad1: u32,
    forward: [f32; 3],
    _pad2: u32,
    pos: [f32; 3],
    _pad3: u32,

    yaw: f32,
    pitch: f32,

    sensor_h: f32,

    aspect_ratio: f32,

    focal_length: f32,
    _pad4: [u32; 3],

}

// const _: () = assert!(std::mem::size_of::<Camera>() % 16 == 0);
const _: () = assert!(std::mem::size_of::<CameraLean>() % 16 == 0);
const _: () = assert!(std::mem::size_of::<CameraBasis>() % 16 == 0);
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
        let sensitivity = 0.01;

        let mut camera = Self {
            up_vector,
            position,
            look_at,

            forward,
            right,
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
            yaw: 0.0,
            pitch: 0.0,
            sensitivity,
        };

        camera.update_camera_config();
        camera
    }
}

impl Camera {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn compute_sensor(&self) -> CameraLean {
        let lookat = Vector3f::from(self.look_at);
        let pos = Vector3f::from(self.position);
        let up_v = Vector3f::from(self.up_vector);

        let forward = (lookat - pos).normalize();
        let right = up_v.cross(&forward).normalize();
        let up = forward.cross(&right);

        //
        let sensor_height = self.sensor_height;
        let sensor_width = sensor_height * self.aspect_ratio;

        let pic_width = self.picture_width;
        let pic_height = (pic_width as f32 / self.aspect_ratio) as u32;

        let sensor_u = right * -sensor_width;
        let sensor_v = up * -sensor_height;
        let pixel_delta_u = sensor_u / pic_width as f32;
        let pixel_delta_v = sensor_v / pic_height as f32;
        let sensor_corner =
            pos + forward * self.focal_length - ((sensor_u + sensor_v) * 0.5);

        let pixeloo = sensor_corner + ((pixel_delta_u + pixel_delta_v) * 0.5);

        CameraLean {
            pixeloo: pixeloo.into(),
            _pad0: 0,
            pixel_delta_u: pixel_delta_u.into(),
            _pad1: 0,
            pixel_delta_v: pixel_delta_v.into(),
            _pad2: 0,
            pos: pos.into(),
            _pad3: 0,
        }
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

    #[allow(dead_code)]
    pub fn cumul_orientation_delta(&mut self, (x, y): (f64, f64)) { 
        self.yaw += self.sensitivity * x as f32;
        self.pitch += self.sensitivity * y as f32;
    }

    #[allow(dead_code)]
    pub fn reset_orientation_delta(&mut self) { 
        self.yaw = 0.0;
        self.pitch = 0.0;
    }
}

impl From<Camera> for CameraBasis {
    fn from(camera: Camera) -> Self {


        let lookat = Vector3f::from(camera.look_at);
        let pos = Vector3f::from(camera.position);
        let up_v = Vector3f::from(camera.up_vector);

        let forward = (lookat - pos).normalize();
        let right = up_v.cross(&forward).normalize();
        let up = forward.cross(&right);


        CameraBasis {
            right: right.into(),
            _pad0: 0,
            up: up.into(),
            _pad1: 0,
            forward: forward.into(),
            _pad2: 0,
            pos: pos.into(),
            _pad3: 0,

            yaw: camera.yaw,
            pitch: camera.pitch,
            sensor_h: camera.sensor_height,
            aspect_ratio: camera.aspect_ratio,
            focal_length: camera.focal_length,
            _pad4: [0; 3],
        }
    }
}
