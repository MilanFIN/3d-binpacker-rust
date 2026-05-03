use crate::common::box_spec::BinBox;
use crate::common::bin::Bin;
use crate::solver::solver_interface::Solver;
use crate::solver::solver_properties::SolverProperties;
use crate::solver::parallelsolvers::ParallelSolver;
use ocl::{ProQue, Buffer, MemFlags, Device, Platform};

pub struct OpenCLSolver {
    kernel_file_name: String,
    kernel_function_name: String,
    display_name: String,
    reference_solver: Box<dyn Solver + Send + Sync>,
    device_index: Option<usize>,
    
    bin_template: Option<Bin>,
    rotation_axes: Option<Vec<i32>>,
    kernel_source: String,
    
    pro_que: Option<ProQue>,
}

impl OpenCLSolver {
    pub fn new(
        kernel_file_name: &str,
        kernel_function_name: &str,
        display_name: &str,
        reference_solver: Box<dyn Solver + Send + Sync>,
        device_index: Option<usize>,
    ) -> Self {
        let kernel_source = Self::load_kernel_source(kernel_file_name);
        Self {
            kernel_file_name: kernel_file_name.to_string(),
            kernel_function_name: kernel_function_name.to_string(),
            display_name: display_name.to_string(),
            reference_solver,
            device_index,
            bin_template: None,
            rotation_axes: None,
            kernel_source,
            pro_que: None,
        }
    }
    
    fn load_kernel_source(name: &str) -> String {
        match name {
            "firstfit_ems.cl.template" => include_str!("../../kernels/firstfit_ems.cl.template").to_string(),
            "bestfit_ems.cl.template" => include_str!("../../kernels/bestfit_ems.cl.template").to_string(),
            "firstfit_complete.cl.template" => include_str!("../../kernels/firstfit_complete.cl.template").to_string(),
            "bestfit_complete.cl.template" => include_str!("../../kernels/bestfit_complete.cl.template").to_string(),
            _ => panic!("Unknown kernel {}", name),
        }
    }

    fn build_pro_que(&self, source: &str) -> ProQue {
        let mut builder = ProQue::builder();
        builder.src(source);
        
        if let Some(idx) = self.device_index {
            let platform = Platform::default();
            if let Ok(devices) = Device::list_all(platform) {
                if idx < devices.len() {
                    builder.device(devices[idx]);
                } else {
                    eprintln!("Warning: Device index {} is out of bounds (found {} devices). Using default.", idx, devices.len());
                }
            }
        }
        
        builder.build().expect("Failed to build OpenCL ProQue")
    }
}

impl ParallelSolver for OpenCLSolver {
    fn is_template(&self) -> bool {
        self.kernel_file_name.ends_with(".template")
    }

    fn is_compiled(&self) -> bool {
        self.pro_que.is_some()
    }

    fn compile_kernel(&mut self, max_bins: usize, max_spaces: usize) {
        if self.pro_que.is_some() {
            return;
        }

        let source = self.kernel_source
            .replace("{{MAX_BINS}}", &max_bins.to_string())
            .replace("{{MAX_SPACES_PER_BIN}}", &max_spaces.to_string());

        let pro_que = self.build_pro_que(&source);

        self.pro_que = Some(pro_que);
    }

    fn init(&mut self, properties: &SolverProperties) {
        self.bin_template = Some(properties.bin.clone());
        self.rotation_axes = Some(properties.rotation_axes.clone());
        
        if !self.is_template() {
            let pro_que = self.build_pro_que(&self.kernel_source);
            self.pro_que = Some(pro_que);
        }
    }

    fn get_reference_solver(&mut self) -> Option<&mut dyn Solver> {
        Some(&mut *self.reference_solver)
    }

    fn solve(&mut self, boxes: &[BinBox], orders: &[Vec<usize>]) -> Vec<f64> {
        let num_boxes = boxes.len();
        let num_orders = orders.len();

        if num_boxes == 0 || num_orders == 0 {
            return vec![];
        }

        let mut box_data = vec![0.0f32; num_boxes * 4];
        for (i, b) in boxes.iter().enumerate() {
            box_data[i * 4 + 0] = b.size.x as f32;
            box_data[i * 4 + 1] = b.size.y as f32;
            box_data[i * 4 + 2] = b.size.z as f32;
            box_data[i * 4 + 3] = b.weight as f32;
        }

        let mut order_data = vec![0i32; num_orders * num_boxes];
        for (i, order) in orders.iter().enumerate() {
            for (j, &idx) in order.iter().enumerate() {
                order_data[i * num_boxes + j] = idx as i32;
            }
        }

        let pro_que = self.pro_que.as_ref().expect("OpenCL not compiled");

        let boxes_buf = Buffer::<f32>::builder()
            .queue(pro_que.queue().clone())
            .flags(MemFlags::new().read_only().copy_host_ptr())
            .len(box_data.len())
            .copy_host_slice(&box_data)
            .build()
            .unwrap();

        let orders_buf = Buffer::<i32>::builder()
            .queue(pro_que.queue().clone())
            .flags(MemFlags::new().read_only().copy_host_ptr())
            .len(order_data.len())
            .copy_host_slice(&order_data)
            .build()
            .unwrap();

        let mut scores_buf = Buffer::<f32>::builder()
            .queue(pro_que.queue().clone())
            .flags(MemFlags::new().write_only())
            .len(num_orders)
            .build()
            .unwrap();

        let mut rotation_mask = 0i32;
        if let Some(axes) = &self.rotation_axes {
            if axes.contains(&0) { rotation_mask |= 1; }
            if axes.contains(&1) { rotation_mask |= 2; }
            if axes.contains(&2) { rotation_mask |= 4; }
        }

        let bin = self.bin_template.as_ref().unwrap();

        let kernel = pro_que.kernel_builder(&self.kernel_function_name)
            .arg(&boxes_buf)
            .arg(&orders_buf)
            .arg(&scores_buf)
            .arg(&(num_boxes as i32))
            .arg(&(bin.w as f32))
            .arg(&(bin.h as f32))
            .arg(&(bin.d as f32))
            .arg(&(bin.max_weight as f32))
            .arg(&rotation_mask)
            .global_work_size(num_orders)
            .build()
            .expect("Failed to build kernel");

        unsafe {
            kernel.enq().unwrap();
        }

        let mut scores = vec![0.0f32; num_orders];
        scores_buf.read(&mut scores).enq().unwrap();

        scores.into_iter().map(|s| s as f64).collect()
    }
}
