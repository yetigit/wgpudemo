use nalgebra::Vector3;
use crate::camera::Camera;

type Vector3f = Vector3<f32>;

#[derive(Debug, Copy, Clone)]
pub enum CameraDirection { 
    Forward,
    Backward,
    Right,
    Left,
    Up,
    Down,
}

#[derive(Debug, Copy, Clone)]
pub enum CameraNavMode { 
    Idle,
    FPS,
    Look,
    Pan,
}

#[derive(Copy, Clone, Debug)]
pub struct CameraController {
    pub speed: f32,
    pub sensitivity: f32,
    pub fwd_move: f32,
    pub right_move: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub yaw: f32,
    pub pitch: f32,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WgslCameraControls {
     speed: f32,
     sensitivity: f32,
     fwd_move: f32,
     right_move: f32,
     pan_x: f32,
     pan_y: f32,
     yaw: f32,
     pitch: f32,
}

const _: () = assert!(std::mem::size_of::<WgslCameraControls>() % 16 == 0);

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            speed,
            sensitivity,
            fwd_move: 0.0,
            right_move: 0.0,
            pan_x: 0.0,
            pan_y: 0.0,
            yaw : (-90.0 as f32).to_radians(),
            pitch: 0.0,
        }
    }

    pub fn process_move(&mut self, dir: CameraDirection) {
        match dir { 
             CameraDirection::Forward=> self.fwd_move += self.speed,
             CameraDirection::Backward=> self.fwd_move += -self.speed,
             CameraDirection::Right=> self.right_move += -self.speed,
             CameraDirection::Left=> self.right_move += self.speed,
             _ => {}
        }
    }

    pub fn process_pan(&mut self, pos: (f64, f64)) {
        self.pan_x += 1.0 * pos.0 as f32;
        self.pan_y += 1.0 * pos.1 as f32;
    }

    pub fn process_look(&mut self, (x, y): (f64, f64)) {
        self.yaw += self.sensitivity * x as f32;
        self.pitch += self.sensitivity * y as f32;
    }


    pub fn update_camera(&mut self, camera: &mut Camera) {
        // Apply Move
        if self.fwd_move != 0.0 || self.right_move != 0.0 {
            let forward : Vector3f = camera.forward.into();
            let right : Vector3f = camera.right.into();
            let mut pos : Vector3f =  camera.position.into();
            pos += forward.normalize() * self.fwd_move;
            pos += right.normalize() * self.right_move;
            camera.position = pos.into();
            
            // Reset per-frame move triggers
            self.fwd_move = 0.0;
            self.right_move = 0.0;
        }

        // Apply Pan
        if self.pan_x != 0.0 || self.pan_y != 0.0 {
            let right : Vector3f = camera.right.into();
            let up : Vector3f = camera.up.into();
            let mut pos : Vector3f =  camera.position.into();
            pos += right.normalize() * self.pan_x;
            pos += up.normalize() * self.pan_y;
            camera.position = pos.into();
            
            self.pan_x = 0.0;
            self.pan_y = 0.0;
        }
        // 

      // var fp: vec3<f32> = camera.forward * sin(yaw) + camera.right * cos(yaw);

        // let base_fwd = Vector3f::from(camera.forward);
        // let base_right = Vector3f::from(camera.right);
        // let base_up = Vector3f::from(camera.up);

        // let mut fp = base_fwd * self.yaw.sin() + base_right * self.yaw.cos();
        // let rp = -base_up.cross(&base_fwd);

        // let fp = base_up * -self.pitch.sin() + fp * -self.pitch.cos();
        // let upp = fp.cross(&rp);

        // let right = rp.normalize();
        // let forward = fp.normalize();
        // let up = upp.normalize();
    }
}

impl From<CameraController> for WgslCameraControls {
    fn from(controller: CameraController) -> Self {
        WgslCameraControls {
            speed: controller.speed,
            sensitivity: controller.sensitivity,
            fwd_move: controller.fwd_move,
            right_move: controller.right_move,
            pan_x: controller.pan_x,
            pan_y: controller.pan_y,
            yaw: controller.yaw,
            pitch: controller.pitch,
        }
    }
}
