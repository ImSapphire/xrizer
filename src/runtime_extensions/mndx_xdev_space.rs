mod sys;

use std::{ffi::CStr, ptr::addr_of_mut};

use sys::{
    XrCreateXDevListInfoMNDX, XrCreateXDevSpaceInfoMNDX, XrGetXDevInfoMNDX, XrXDevIdMNDX,
    XrXDevListMNDX, XrXDevPropertiesMNDX,
};

use openxr as xr;

pub const XR_MNDX_XDEV_SPACE_EXTENSION_NAME: &str = "XR_MNDX_xdev_space";

pub struct XdevSpaceExtension {
    xr_mndx_xdev_space: sys::XdevSpaceExtension,
}

pub struct Xdev {
    pub _id: XrXDevIdMNDX,
    pub properties: XrXDevPropertiesMNDX,
    pub space: Option<xr::Space>,
    pub serial: &'static CStr,
}

impl PartialEq for Xdev {
    fn eq(&self, other: &Self) -> bool {
        self._id == other._id
    }
}

impl Xdev {
    pub fn new(
        _id: XrXDevIdMNDX,
        properties: XrXDevPropertiesMNDX,
        space: Option<xr::Space>,
    ) -> Self {
        Self {
            _id,
            properties,
            space,
            serial: unsafe { CStr::from_ptr(properties.serial.as_ptr()) },
        }
    }
}

impl XdevSpaceExtension {
    pub fn new(instance: &xr::Instance) -> xr::Result<Self> {
        Ok(Self {
            xr_mndx_xdev_space: sys::XdevSpaceExtension::new(instance.as_raw())?,
        })
    }

    pub fn enumerate_xdevs(
        &self,
        session: &xr::Session<xr::AnyGraphics>,
        max_generic_trackers: usize,
    ) -> xr::Result<Vec<Xdev>> {
        let mut xdev_list = XrXDevListMNDX::default();
        let create_info = XrCreateXDevListInfoMNDX::default();

        let mut xdev_ids = vec![0; max_generic_trackers];
        let mut xdev_id_count = 0;

        log::info!("Create XDev List");

        self.xr_mndx_xdev_space
            .create_xdev_list(session.as_raw(), &create_info, &mut xdev_list)?;

        log::info!("Enumerate XDevs");

        self.xr_mndx_xdev_space.enumerate_xdevs(
            xdev_list,
            max_generic_trackers as u32,
            addr_of_mut!(xdev_id_count),
            xdev_ids.as_mut_ptr(),
        )?;

        xdev_ids.truncate(xdev_id_count as usize);

        let mut current_properties = XrXDevPropertiesMNDX::default();
        let mut current_get_info = XrGetXDevInfoMNDX::default();
        let mut space_create_info =
            XrCreateXDevSpaceInfoMNDX::new(xdev_list, 0, xr::Posef::IDENTITY);

        let xdevs = xdev_ids
            .iter()
            .map(|&id| {
                current_get_info.id = id;
                space_create_info.id = id;

                self.xr_mndx_xdev_space.get_xdev_properties(
                    xdev_list,
                    &current_get_info,
                    &mut current_properties,
                )?;

                if current_properties.can_create_space() {
                    let mut raw_space = xr::sys::Space::default();

                    self.xr_mndx_xdev_space.create_xdev_space(
                        session.as_raw(),
                        &space_create_info,
                        &mut raw_space,
                    )?;

                    let space =
                        unsafe { xr::Space::reference_from_raw(session.to_owned(), raw_space) };

                    Ok(Xdev::new(id, current_properties, Some(space)))
                } else {
                    Ok(Xdev::new(id, current_properties, None))
                }
            })
            .collect::<xr::Result<Vec<Xdev>>>();

        self.xr_mndx_xdev_space.destroy_xdev_list(xdev_list)?;

        xdevs
    }
}
