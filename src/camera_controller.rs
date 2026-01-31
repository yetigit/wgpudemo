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
    const YAW_START: f32 = -90.0 * (std::f32::consts::PI / 180.0);
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            speed,
            sensitivity,
            fwd_move: 0.0,
            right_move: 0.0,
            pan_x: 0.0,
            pan_y: 0.0,
            yaw : CameraController::YAW_START,
            pitch: 0.0,
        }
    }

    pub fn process_move(&mut self, dir: CameraDirection) {
        match dir { 
             CameraDirection::Forward=> self.fwd_move += 1.0 * self.speed,
             CameraDirection::Backward=> self.fwd_move += -1.0 * self.speed,
             CameraDirection::Right=> self.right_move += -1.0 * self.speed,
             CameraDirection::Left=> self.right_move += 1.0 * self.speed,
             _ => {}
        }
    }

    pub fn process_pan(&mut self, (x, y): (f64, f64)) {
        self.pan_x +=  x as f32;
        self.pan_y +=  y as f32;
    }

    pub fn process_look(&mut self, (x, y): (f64, f64)) {
        self.yaw += self.sensitivity * x as f32;
        self.pitch += self.sensitivity * y as f32;
    }

    pub fn set_zero(&mut self) {
        self.yaw = CameraController::YAW_START;
        self.pitch = 0.0;
        self.fwd_move = 0.0;
        self.right_move = 0.0;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
    }

    pub fn update_camera(&mut self, camera: &mut Camera) {
        // Apply Look
        let base_fwd = Vector3f::from(camera.forward);
        let base_right = Vector3f::from(camera.right);
        let base_up = Vector3f::from(camera.up);

        let yaw = self.yaw;
        let pitch = self.pitch;

        let fp = base_fwd * yaw.sin() + base_right * yaw.cos();
        let rp = -base_up.cross(&fp);

        let fp = base_up * -pitch.sin() + fp * -pitch.cos();
        let upp = fp.cross(&rp);

        let right = rp.normalize();
        let forward = fp.normalize();
        let up = upp.normalize();

        let mut pos: Vector3f = camera.position.into();

        // Apply Move
        if self.fwd_move != 0.0 || self.right_move != 0.0 {
            pos += forward * self.fwd_move;
            pos += right * self.right_move;
        }

        // Apply Pan
        if self.pan_x != 0.0 || self.pan_y != 0.0 {
            pos += right * self.pan_x;
            pos += up * self.pan_y;
        }

        camera.position = pos.into();
        camera.forward = forward.into();
        camera.right = right.into();
        camera.up = up.into();
        self.set_zero();
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
