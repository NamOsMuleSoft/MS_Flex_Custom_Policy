use log::info;
use proxy_wasm::types::*;
use std::ptr::null_mut;

pub const AI_SERVICE_NAME: &str = "appinsights";
pub const AI_SERVICE_HOST_SUFFIX: &str = "in.applicationinsights.azure.com";
pub const AI_SERVICE_PATH: &str = "/v2/track";


#[no_mangle]
pub extern "C" fn flex_abi_version_0_1_0() {}

#[no_mangle]
pub extern "C" fn flex_on_policy_initialize() -> bool {

    // Registering Azure Application Insights upstreams

    // only few as example
    let regions = vec![
        "westeurope",
        "eastus"
      ];

    // for each region and n in [0..18]
    for region in regions {
        
        let service_name = format!("{}-{}", AI_SERVICE_NAME, region);

        // generate the application insights endoint url to register
        let url = format!("https://{}.{}{}", region, AI_SERVICE_HOST_SUFFIX, AI_SERVICE_PATH);

        // sets the arguments
        let args: &[&str] = &[&service_name, "default", &url];
        
        // register the endpoint upstream
        match call_foreign_function("flex_create_service", args) {
            Ok(resp) => match resp{
                Some(res)=>info!("RESP: {}",String::from_utf8(res).unwrap()),
                None => info!("NONE")
            }
            Err(e) => info!("E: {:?}", e)
        }
    }
    return true;
}

fn call_foreign_function(name: &str, args: &[&str]) -> Result<Option<Bytes>, Status> {

    let mut return_data: *mut u8 = null_mut();
    let mut return_size: usize = 0;

    let arg = serde_json::to_string(args).map_err(|_| Status::ParseFailure)?;

    unsafe {
        match proxy_call_foreign_function(
            name.as_ptr(),
            name.len(),
            arg.as_ptr(),
            arg.len(),
            &mut return_data,
            &mut return_size,
        ) {
            Status::Ok => {
                if !return_data.is_null() {
                    Ok(Some(Vec::from_raw_parts(
                        return_data,
                        return_size,
                        return_size,
                    )))
                } else {
                    Ok(None)
                }
            }
            Status::NotFound => Ok(None),
            status => panic!("unexpected status: {}", status as u32),
        }
    }
}

#[allow(improper_ctypes)]
extern "C" {
    fn proxy_call_foreign_function(
        name_data: *const u8,
        name_size: usize,
        args_data: *const u8,
        args_size: usize,
        return_data: *mut *mut u8,
        return_size: *mut usize,
    ) -> Status;
}
