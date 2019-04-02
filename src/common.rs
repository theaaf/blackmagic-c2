#[derive(Clone, Deserialize, Serialize, Message)]
pub enum Message {
    AgentState { id: String, state: AgentState },
    ShellInit { id: String },
    ShellClose { id: String },
    ShellOutput { id: String, bytes: Vec<u8> },
    ShellInput { id: String, bytes: Vec<u8> },
    HyperDeckCommand { id: String, ip_address: String, command: String },
    HyperDeckCommandError { id: String, description: String },
    HyperDeckCommandResponse { id: String, response: HyperDeckCommandResponse },
}

#[derive(Clone, Deserialize, Serialize, GraphQLObject)]
pub struct AgentState {
    pub network_devices: Vec<NetworkDevice>,
    pub decklink_devices: Vec<DeckLinkDevice>,
}

#[derive(Clone, Deserialize, Serialize, GraphQLObject)]
pub struct DeckLinkDevice {
    pub model_name: String,
    pub attributes: DeckLinkDeviceAttributes,
    pub input: DeckLinkDeviceInput,
    pub output: DeckLinkDeviceOutput,
    pub status: DeckLinkDeviceStatus,
}

#[derive(Clone, Deserialize, Serialize, GraphQLObject)]
pub struct HyperDeckDetails {
    pub model_name: String,
    pub protocol_version: String,
    pub unique_id: String,
}

#[derive(Clone, Deserialize, Serialize, GraphQLObject)]
pub struct HyperDeckCommandResponse {
    pub code: i32,
    pub text: String,
    pub payload: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub enum NetworkDeviceDetails {
    HyperDeckDetails(HyperDeckDetails),
}

juniper::graphql_union!(NetworkDeviceDetails: () where Scalar = <S> |&self| {
    instance_resolvers: |_| {
        &HyperDeckDetails => match *self { NetworkDeviceDetails::HyperDeckDetails(ref v) => Some(v) },
    }
});

#[derive(Clone, Deserialize, Serialize, GraphQLObject)]
pub struct NetworkDevice {
    pub ip_address: String,
    pub mac_address: String,
    pub details: Option<NetworkDeviceDetails>,
}

#[derive(Clone, Deserialize, Serialize, GraphQLObject)]
pub struct DeckLinkDisplayMode {
    pub id: i32,
    pub name: String,
}

#[derive(Clone, Deserialize, Serialize, GraphQLObject)]
pub struct DeckLinkVideoIOSupport {
    pub capture: bool,
    pub playback: bool,
}

#[derive(Clone, Deserialize, Serialize, GraphQLObject)]
pub struct DeckLinkDeviceInput {
    pub display_modes: Vec<DeckLinkDisplayMode>,
}

#[derive(Clone, Deserialize, Serialize, GraphQLObject)]
pub struct DeckLinkDeviceOutput {
    pub display_modes: Vec<DeckLinkDisplayMode>,
}

#[derive(Clone, Deserialize, Serialize, GraphQLObject)]
pub struct DeckLinkDeviceBusyState {
    pub capture_busy: bool,
    pub playback_busy: bool,
    pub serial_port_busy: bool,
}

#[derive(Clone, Deserialize, Serialize, GraphQLObject)]
pub struct DeckLinkDeviceStatus {
    pub video_input_signal_locked: Option<bool>,
    pub reference_signal_locked: Option<bool>,
    pub received_edid: Option<bool>,
    pub detected_video_input_mode: Option<DeckLinkDisplayMode>,
    pub current_video_input_mode: Option<DeckLinkDisplayMode>,
    pub current_video_output_mode: Option<DeckLinkDisplayMode>,
    pub pci_express_link_width: Option<i32>,
    pub pci_express_link_speed: Option<i32>,
    pub busy: Option<DeckLinkDeviceBusyState>,
    pub device_temperature: Option<i32>,
}

#[derive(Clone, Deserialize, Serialize, GraphQLObject)]
pub struct DeckLinkDeviceAttributes {
    pub supports_internal_keying: Option<bool>,
    pub supports_external_keying: Option<bool>,
    pub supports_hd_keying: Option<bool>,
    pub supports_input_format_detection: Option<bool>,
    pub has_reference_input: Option<bool>,
    pub has_serial_port: Option<bool>,
    pub has_analog_video_output_gain: Option<bool>,
    pub can_only_adjust_overall_video_output_gain: Option<bool>,
    pub has_video_input_antialiasing_filter: Option<bool>,
    pub has_bypass: Option<bool>,
    pub supports_clock_timing_adjustment: Option<bool>,
    pub supports_full_duplex: Option<bool>,
    pub supports_full_frame_reference_input_timing_offset: Option<bool>,
    pub supports_smpte_level_a_output: Option<bool>,
    pub supports_dual_link_sdi: Option<bool>,
    pub supports_quad_link_sdi: Option<bool>,
    pub supports_idle_output: Option<bool>,
    pub has_ltc_timecode_input: Option<bool>,
    pub supports_duplex_mode_configuration: Option<bool>,
    pub supports_hdr_metadata: Option<bool>,
    pub supports_colorspace_metadata: Option<bool>,
    pub maximum_audio_channels: Option<i32>,
    pub maximum_analog_audio_input_channels: Option<i32>,
    pub maximum_analog_audio_output_channels: Option<i32>,
    pub number_of_subdevices: Option<i32>,
    pub subdevice_index: Option<i32>,
    pub persistent_id: Option<i32>,
    pub device_group_id: Option<i32>,
    pub topological_id: Option<i32>,
    pub video_io_support: Option<DeckLinkVideoIOSupport>,
    pub audio_input_rca_channel_count: Option<i32>,
    pub audio_input_xlr_channel_count: Option<i32>,
    pub audio_output_rca_channel_count: Option<i32>,
    pub audio_output_xlr_channel_count: Option<i32>,
    pub paired_device_persistent_id: Option<i32>,
    pub video_input_gain_minimum: Option<f64>,
    pub video_input_gain_maximum: Option<f64>,
    pub video_output_gain_minimum: Option<f64>,
    pub video_output_gain_maximum: Option<f64>,
    pub microphone_input_gain_minimum: Option<f64>,
    pub microphone_input_gain_maximum: Option<f64>,
    pub serial_port_device_name: Option<String>,
    pub vendor_name: Option<String>,
    pub display_name: Option<String>,
    pub model_name: Option<String>,
    pub device_handle: Option<String>,
}
