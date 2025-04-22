use openvr as vr;
use openxr as xr;

use crate::openxr_data::{Hand, OpenXrData, SessionData};

use super::tracked_device::{TrackedDeviceType, XrTrackedDevice};

use log::trace;

impl XrTrackedDevice {
    pub(super) fn get_controller_pose(
        &self,
        xr_data: &OpenXrData<impl crate::openxr_data::Compositor>,
        session_data: &SessionData,
        origin: vr::ETrackingUniverseOrigin,
    ) -> Option<vr::TrackedDevicePose_t> {
        let pose_data = session_data.input_data.pose_data.get().unwrap();
        let space = match self.get_controller_hand()? {
            Hand::Left => &pose_data.left_space,
            Hand::Right => &pose_data.right_space,
        };

        let (location, velocity) = if let Some(raw) = space.try_get_or_init_raw(
            &self.get_interaction_profile(),
            session_data,
            &pose_data,
        ) {
            raw.relate(
                session_data.get_space_for_origin(origin),
                xr_data.display_time.get(),
            )
            .ok()?
        } else {
            trace!("Failed to get raw space, returning empty pose");
            (xr::SpaceLocation::default(), xr::SpaceVelocity::default())
        };

        Some(vr::space_relation_to_openvr_pose(location, velocity))
    }

    pub fn get_controller_hand(&self) -> Option<Hand> {
        match self.get_type() {
            TrackedDeviceType::Controller { hand, .. } => Some(hand),
            _ => None,
        }
    }
}
