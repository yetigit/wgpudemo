# A WebGPU Raytracer for the web using Rust Web Assembly

![chrome_qeRsrNqd8j](https://github.com/user-attachments/assets/a0420518-1c90-4558-8576-241369a97934)

## Instructions
```
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
run.ps1
```
Copy the address of the host, paste it in the `Google Chrome` browser, and *voila*.

---

**TODO**
- [ ] triangle mesh support, including BVH traversal in the shader
- [ ] DOF pass
- [ ] Denoising pass using denoiser models
