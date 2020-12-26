use crate::rtp_parameters::{
    MediaKind, MimeType, MimeTypeVideo, RtcpFeedback, RtcpParameters, RtpCapabilities,
    RtpCapabilitiesFinalized, RtpCodecCapability, RtpCodecCapabilityFinalized, RtpCodecParameters,
    RtpCodecParametersParameters, RtpCodecParametersParametersValue, RtpEncodingParameters,
    RtpEncodingParametersRtx, RtpHeaderExtensionDirection, RtpHeaderExtensionParameters,
    RtpHeaderExtensionUri, RtpParameters,
};
use crate::scalability_modes::ScalabilityMode;
use crate::supported_rtp_capabilities;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::mem;
use std::num::{NonZeroU32, NonZeroU8};
use std::ops::Deref;
use thiserror::Error;

const DYNAMIC_PAYLOAD_TYPES: &[u8] = &[
    100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118,
    119, 120, 121, 122, 123, 124, 125, 126, 127, 96, 97, 98, 99,
];

#[doc(hidden)]
#[derive(Debug, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RtpMappingCodec {
    pub payload_type: u8,
    pub mapped_payload_type: u8,
}

#[doc(hidden)]
#[derive(Debug, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RtpMappingEncoding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssrc: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scalability_mode: Option<String>,
    pub mapped_ssrc: u32,
}

#[doc(hidden)]
#[derive(Debug, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Deserialize, Serialize)]
pub struct RtpMapping {
    pub codecs: Vec<RtpMappingCodec>,
    pub encodings: Vec<RtpMappingEncoding>,
}

/// Error caused by invalid RTP parameters.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum RtpParametersError {
    /// Invalid codec apt parameter.
    #[error("Invalid codec apt parameter {0}")]
    InvalidAptParameter(String),
}

/// Error caused by invalid RTP capabilities.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum RtpCapabilitiesError {
    /// Media codec not supported.
    #[error("Media codec not supported [mime_type:{mime_type:?}")]
    UnsupportedCodec { mime_type: MimeType },
    /// Cannot allocate more dynamic codec payload types.
    #[error("Cannot allocate more dynamic codec payload types")]
    CannotAllocate,
    /// Invalid codec apt parameter.
    #[error("Invalid codec apt parameter {0}")]
    InvalidAptParameter(String),
    /// Duplicated preferred payload type
    #[error("Duplicated preferred payload type {0}")]
    DuplicatedPreferredPayloadType(u8),
}

/// Error caused by invalid or unsupported RTP parameters given.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum RtpParametersMappingError {
    /// Unsupported codec.
    #[error("Unsupported codec [mime_type:{mime_type:?}, payloadType:{payload_type}]")]
    UnsupportedCodec {
        mime_type: MimeType,
        payload_type: u8,
    },
    /// No RTX codec for capability codec PT.
    #[error("No RTX codec for capability codec PT {preferred_payload_type}")]
    UnsupportedRTXCodec { preferred_payload_type: u8 },
    /// Missing media codec found for RTX PT.
    #[error("Missing media codec found for RTX PT {payload_type}")]
    MissingMediaCodecForRTX { payload_type: u8 },
}

/// Error caused by bad consumer RTP parameters.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum ConsumerRtpParametersError {
    /// Invalid capabilities
    #[error("Invalid capabilities: {0}")]
    InvalidCapabilities(RtpCapabilitiesError),
    /// No compatible media codecs
    #[error("No compatible media codecs")]
    NoCompatibleMediaCodecs,
}

fn generate_ssrc() -> u32 {
    fastrand::u32(100000000..999999999)
}

/// Validates RtpParameters.
pub(crate) fn validate_rtp_parameters(
    rtp_parameters: &RtpParameters,
) -> Result<(), RtpParametersError> {
    for codec in rtp_parameters.codecs.iter() {
        validate_rtp_codec_parameters(codec)?;
    }

    Ok(())
}

/// Validates RtpCodecParameters.
fn validate_rtp_codec_parameters(codec: &RtpCodecParameters) -> Result<(), RtpParametersError> {
    for (key, value) in codec.parameters().iter() {
        // Specific parameters validation.
        if key.as_str() == "apt" {
            match value {
                RtpCodecParametersParametersValue::Number(_) => {
                    // Good
                }
                RtpCodecParametersParametersValue::String(string) => {
                    return Err(RtpParametersError::InvalidAptParameter(string.clone()));
                }
            }
        }
    }

    Ok(())
}

// Validates RtpCodecCapability.
fn validate_rtp_codec_capability(codec: &RtpCodecCapability) -> Result<(), RtpCapabilitiesError> {
    for (key, value) in codec.parameters().iter() {
        // Specific parameters validation.
        if key.as_str() == "apt" {
            match value {
                RtpCodecParametersParametersValue::Number(_) => {
                    // Good
                }
                RtpCodecParametersParametersValue::String(string) => {
                    return Err(RtpCapabilitiesError::InvalidAptParameter(string.clone()));
                }
            }
        }
    }

    Ok(())
}

/// Validates RtpCapabilities.
pub(crate) fn validate_rtp_capabilities(
    caps: &RtpCapabilities,
) -> Result<(), RtpCapabilitiesError> {
    for codec in caps.codecs.iter() {
        validate_rtp_codec_capability(codec)?;
    }

    Ok(())
}

/// Generate RTP capabilities for the Router based on the given media codecs and mediasoup supported
/// RTP capabilities.
pub(crate) fn generate_router_rtp_capabilities(
    mut media_codecs: Vec<RtpCodecCapability>,
) -> Result<RtpCapabilitiesFinalized, RtpCapabilitiesError> {
    let supported_rtp_capabilities = supported_rtp_capabilities::get_supported_rtp_capabilities();

    validate_rtp_capabilities(&supported_rtp_capabilities)?;

    let mut dynamic_payload_types = Vec::from(DYNAMIC_PAYLOAD_TYPES);
    let mut caps = RtpCapabilitiesFinalized {
        codecs: vec![],
        header_extensions: supported_rtp_capabilities.header_extensions,
        fec_mechanisms: vec![],
    };

    for media_codec in media_codecs.iter_mut() {
        validate_rtp_codec_capability(media_codec)?;

        let codec = match supported_rtp_capabilities
            .codecs
            .iter()
            .find(|supported_codec| {
                match_codecs(media_codec.deref().into(), (*supported_codec).into(), false).is_ok()
            }) {
            Some(codec) => codec,
            None => {
                return Err(RtpCapabilitiesError::UnsupportedCodec {
                    mime_type: media_codec.mime_type(),
                });
            }
        };

        let preferred_payload_type = match media_codec.preferred_payload_type() {
            // If the given media codec has preferred_payload_type, keep it.
            Some(preferred_payload_type) => {
                // Also remove the payload_type from the list of available dynamic values.
                dynamic_payload_types.retain(|&pt| pt != preferred_payload_type);

                preferred_payload_type
            }
            None => {
                match codec.preferred_payload_type() {
                    // Otherwise if the supported codec has preferredPayloadType, use it.
                    Some(preferred_payload_type) => {
                        // No need to remove it from the list since it's not a dynamic value.
                        preferred_payload_type
                    }
                    // Otherwise choose a dynamic one.
                    None => {
                        if dynamic_payload_types.is_empty() {
                            return Err(RtpCapabilitiesError::CannotAllocate);
                        }
                        // Take the first available payload type and remove it from the list.
                        dynamic_payload_types.remove(0)
                    }
                }
            }
        };

        // Ensure there is not duplicated preferredPayloadType values.
        for codec in caps.codecs.iter() {
            if codec.preferred_payload_type() == preferred_payload_type {
                return Err(RtpCapabilitiesError::DuplicatedPreferredPayloadType(
                    preferred_payload_type,
                ));
            }
        }

        let codec_finalized = match codec {
            RtpCodecCapability::Audio {
                mime_type,
                preferred_payload_type: _,
                clock_rate,
                channels,
                parameters,
                rtcp_feedback,
            } => RtpCodecCapabilityFinalized::Audio {
                mime_type: *mime_type,
                preferred_payload_type,
                clock_rate: *clock_rate,
                channels: *channels,
                parameters: {
                    // Merge the media codec parameters.
                    let mut parameters = parameters.clone();
                    parameters.extend(mem::take(media_codec.parameters_mut()));
                    parameters
                },
                rtcp_feedback: rtcp_feedback.clone(),
            },
            RtpCodecCapability::Video {
                mime_type,
                preferred_payload_type: _,
                clock_rate,
                parameters,
                rtcp_feedback,
            } => RtpCodecCapabilityFinalized::Video {
                mime_type: *mime_type,
                preferred_payload_type,
                clock_rate: *clock_rate,
                parameters: {
                    // Merge the media codec parameters.
                    let mut parameters = parameters.clone();
                    parameters.extend(mem::take(media_codec.parameters_mut()));
                    parameters
                },
                rtcp_feedback: rtcp_feedback.clone(),
            },
        };

        // Add a RTX video codec if video.
        if matches!(codec_finalized, RtpCodecCapabilityFinalized::Video {..}) {
            if dynamic_payload_types.is_empty() {
                return Err(RtpCapabilitiesError::CannotAllocate);
            }
            // Take the first available payload_type and remove it from the list.
            let payload_type = dynamic_payload_types.remove(0);

            let rtx_codec = RtpCodecCapabilityFinalized::Video {
                mime_type: MimeTypeVideo::RTX,
                preferred_payload_type: payload_type,
                clock_rate: codec_finalized.clock_rate(),
                parameters: RtpCodecParametersParameters::from([(
                    "apt",
                    codec_finalized.preferred_payload_type().into(),
                )]),
                rtcp_feedback: vec![],
            };

            // Append to the codec list.
            caps.codecs.push(codec_finalized);
            caps.codecs.push(rtx_codec);
        } else {
            // Append to the codec list.
            caps.codecs.push(codec_finalized);
        }
    }

    Ok(caps)
}

/// Get a mapping of codec payloads and encodings of the given Producer RTP parameters as values
/// expected by the Router.
pub(crate) fn get_producer_rtp_parameters_mapping(
    rtp_parameters: &RtpParameters,
    rtp_capabilities: &RtpCapabilitiesFinalized,
) -> Result<RtpMapping, RtpParametersMappingError> {
    let mut rtp_mapping = RtpMapping::default();

    // Match parameters media codecs to capabilities media codecs.
    let mut codec_to_cap_codec =
        BTreeMap::<&RtpCodecParameters, Cow<RtpCodecCapabilityFinalized>>::new();

    for codec in rtp_parameters.codecs.iter() {
        if codec.is_rtx() {
            continue;
        }

        // Search for the same media codec in capabilities.
        match rtp_capabilities.codecs.iter().find_map(|cap_codec| {
            match_codecs(codec.into(), cap_codec.into(), true)
                .ok()
                .map(|profile_level_id| {
                    // This is rather ugly, but we need to fix `profile-level-id` and this was the
                    // quickest way to do it
                    if let Some(profile_level_id) = profile_level_id {
                        let mut cap_codec = cap_codec.clone();
                        cap_codec
                            .parameters_mut()
                            .insert("profile-level-id", profile_level_id);
                        Cow::Owned(cap_codec)
                    } else {
                        Cow::Borrowed(cap_codec)
                    }
                })
        }) {
            Some(matched_codec_capability) => {
                codec_to_cap_codec.insert(codec, matched_codec_capability);
            }
            None => {
                return Err(RtpParametersMappingError::UnsupportedCodec {
                    mime_type: codec.mime_type(),
                    payload_type: codec.payload_type(),
                });
            }
        }
    }

    // Match parameters RTX codecs to capabilities RTX codecs.
    for codec in rtp_parameters.codecs.iter() {
        if !codec.is_rtx() {
            continue;
        }

        // Search for the associated media codec.
        let associated_media_codec = rtp_parameters.codecs.iter().find(|media_codec| {
            let media_codec_payload_type = media_codec.payload_type();
            let codec_parameters_apt = codec.parameters().get(&"apt".to_string());

            match codec_parameters_apt {
                Some(RtpCodecParametersParametersValue::Number(apt)) => {
                    media_codec_payload_type as u32 == *apt
                }
                _ => false,
            }
        });

        match associated_media_codec {
            Some(associated_media_codec) => {
                let cap_media_codec = codec_to_cap_codec.get(associated_media_codec).unwrap();

                // Ensure that the capabilities media codec has a RTX codec.
                let associated_cap_rtx_codec = rtp_capabilities.codecs.iter().find(|cap_codec| {
                    if !cap_codec.is_rtx() {
                        return false;
                    }

                    let cap_codec_parameters_apt = cap_codec.parameters().get(&"apt".to_string());
                    match cap_codec_parameters_apt {
                        Some(RtpCodecParametersParametersValue::Number(apt)) => {
                            cap_media_codec.preferred_payload_type() as u32 == *apt
                        }
                        _ => false,
                    }
                });

                match associated_cap_rtx_codec {
                    Some(associated_cap_rtx_codec) => {
                        codec_to_cap_codec.insert(codec, Cow::Borrowed(associated_cap_rtx_codec));
                    }
                    None => {
                        return Err(RtpParametersMappingError::UnsupportedRTXCodec {
                            preferred_payload_type: cap_media_codec.preferred_payload_type(),
                        });
                    }
                }
            }
            None => {
                return Err(RtpParametersMappingError::MissingMediaCodecForRTX {
                    payload_type: codec.payload_type(),
                });
            }
        }
    }

    // Generate codecs mapping.
    for (codec, cap_codec) in codec_to_cap_codec {
        rtp_mapping.codecs.push(RtpMappingCodec {
            payload_type: codec.payload_type(),
            mapped_payload_type: cap_codec.preferred_payload_type(),
        });
    }

    // Generate encodings mapping.
    let mut mapped_ssrc: u32 = generate_ssrc();

    for encoding in rtp_parameters.encodings.iter() {
        rtp_mapping.encodings.push(RtpMappingEncoding {
            ssrc: encoding.ssrc,
            rid: encoding.rid.clone(),
            scalability_mode: encoding.scalability_mode.clone(),
            mapped_ssrc,
        });

        mapped_ssrc += 1;
    }

    Ok(rtp_mapping)
}

// Generate RTP parameters to be internally used by Consumers given the RTP parameters of a Producer
// and the RTP capabilities of the Router.
pub(crate) fn get_consumable_rtp_parameters(
    kind: MediaKind,
    params: &RtpParameters,
    caps: &RtpCapabilitiesFinalized,
    rtp_mapping: &RtpMapping,
) -> RtpParameters {
    let mut consumable_params = RtpParameters::default();

    for codec in params.codecs.iter() {
        if codec.is_rtx() {
            continue;
        }

        let consumable_codec_pt = rtp_mapping
            .codecs
            .iter()
            .find(|entry| entry.payload_type == codec.payload_type())
            .unwrap()
            .mapped_payload_type;

        let consumable_codec = match caps
            .codecs
            .iter()
            .find(|cap_codec| cap_codec.preferred_payload_type() == consumable_codec_pt)
            .unwrap()
        {
            RtpCodecCapabilityFinalized::Audio {
                mime_type,
                preferred_payload_type,
                clock_rate,
                channels,
                parameters: _,
                rtcp_feedback,
            } => {
                RtpCodecParameters::Audio {
                    mime_type: *mime_type,
                    payload_type: *preferred_payload_type,
                    clock_rate: *clock_rate,
                    channels: *channels,
                    // Keep the Producer codec parameters.
                    parameters: codec.parameters().clone(),
                    rtcp_feedback: rtcp_feedback.clone(),
                }
            }
            RtpCodecCapabilityFinalized::Video {
                mime_type,
                preferred_payload_type,
                clock_rate,
                parameters: _,
                rtcp_feedback,
            } => {
                RtpCodecParameters::Video {
                    mime_type: *mime_type,
                    payload_type: *preferred_payload_type,
                    clock_rate: *clock_rate,
                    // Keep the Producer codec parameters.
                    parameters: codec.parameters().clone(),
                    rtcp_feedback: rtcp_feedback.clone(),
                }
            }
        };

        let consumable_cap_rtx_codec = caps.codecs.iter().find(|cap_rtx_codec| {
            if !cap_rtx_codec.is_rtx() {
                return false;
            }

            let cap_rtx_codec_parameters_apt = cap_rtx_codec.parameters().get(&"apt".to_string());

            match cap_rtx_codec_parameters_apt {
                Some(RtpCodecParametersParametersValue::Number(apt)) => {
                    *apt as u8 == consumable_codec.payload_type()
                }
                _ => false,
            }
        });

        consumable_params.codecs.push(consumable_codec);

        if let Some(consumable_cap_rtx_codec) = consumable_cap_rtx_codec {
            let consumable_rtx_codec = match consumable_cap_rtx_codec {
                RtpCodecCapabilityFinalized::Audio {
                    mime_type,
                    preferred_payload_type,
                    clock_rate,
                    channels,
                    parameters,
                    rtcp_feedback,
                } => RtpCodecParameters::Audio {
                    mime_type: *mime_type,
                    payload_type: *preferred_payload_type,
                    clock_rate: *clock_rate,
                    channels: *channels,
                    parameters: parameters.clone(),
                    rtcp_feedback: rtcp_feedback.clone(),
                },
                RtpCodecCapabilityFinalized::Video {
                    mime_type,
                    preferred_payload_type,
                    clock_rate,
                    parameters,
                    rtcp_feedback,
                } => RtpCodecParameters::Video {
                    mime_type: *mime_type,
                    payload_type: *preferred_payload_type,
                    clock_rate: *clock_rate,
                    parameters: parameters.clone(),
                    rtcp_feedback: rtcp_feedback.clone(),
                },
            };

            consumable_params.codecs.push(consumable_rtx_codec);
        }
    }

    for cap_ext in caps.header_extensions.iter() {
        // Just take RTP header extension that can be used in Consumers.
        match cap_ext.kind {
            Some(cap_ext_kind) => {
                if cap_ext_kind != kind {
                    continue;
                }
            }
            None => {
                // TODO: Should this really skip "any" extensions?
                continue;
            }
        }
        if !matches!(
            cap_ext.direction,
            RtpHeaderExtensionDirection::SendRecv | RtpHeaderExtensionDirection::SendOnly
        ) {
            continue;
        }

        let consumable_ext = RtpHeaderExtensionParameters {
            uri: cap_ext.uri,
            id: cap_ext.preferred_id,
            encrypt: cap_ext.preferred_encrypt,
        };

        consumable_params.header_extensions.push(consumable_ext);
    }

    for (consumable_encoding, mapped_ssrc) in params.encodings.iter().zip(
        rtp_mapping
            .encodings
            .iter()
            .map(|encoding| encoding.mapped_ssrc),
    ) {
        let mut consumable_encoding = consumable_encoding.clone();
        // Remove useless fields.
        consumable_encoding.rid.take();
        consumable_encoding.rtx.take();
        consumable_encoding.codec_payload_type.take();

        // Set the mapped ssrc.
        consumable_encoding.ssrc = Some(mapped_ssrc);

        consumable_params.encodings.push(consumable_encoding);
    }

    consumable_params.rtcp = RtcpParameters {
        cname: params.rtcp.cname.clone(),
        reduced_size: true,
        mux: Some(true),
    };

    consumable_params
}

/// Check whether the given RTP capabilities can consume the given Producer.
pub(crate) fn can_consume(
    consumable_params: &RtpParameters,
    caps: &RtpCapabilities,
) -> Result<bool, RtpCapabilitiesError> {
    validate_rtp_capabilities(&caps)?;

    let mut matching_codecs = Vec::<&RtpCodecParameters>::new();

    for codec in consumable_params.codecs.iter() {
        if caps
            .codecs
            .iter()
            .any(|cap_codec| match_codecs(cap_codec.deref().into(), codec.into(), true).is_ok())
        {
            matching_codecs.push(codec);
        }
    }

    // Ensure there is at least one media codec.
    Ok(matching_codecs
        .get(0)
        .map(|codec| !codec.is_rtx())
        .unwrap_or_default())
}

/// Generate RTP parameters for a specific Consumer.
///
/// It reduces encodings to just one and takes into account given RTP capabilities to reduce codecs,
/// codecs' RTCP feedback and header extensions, and also enables or disabled RTX.
pub(crate) fn get_consumer_rtp_parameters(
    consumable_params: &RtpParameters,
    caps: RtpCapabilities,
) -> Result<RtpParameters, ConsumerRtpParametersError> {
    let mut consumer_params = RtpParameters::default();
    consumer_params.rtcp = consumable_params.rtcp.clone();

    for cap_codec in caps.codecs.iter() {
        validate_rtp_codec_capability(cap_codec)
            .map_err(ConsumerRtpParametersError::InvalidCapabilities)?;
    }

    let mut rtx_supported = false;

    for mut codec in consumable_params.codecs.clone() {
        if let Some(matched_cap_codec) = caps
            .codecs
            .iter()
            .find(|cap_codec| match_codecs(cap_codec.deref().into(), (&codec).into(), true).is_ok())
        {
            *codec.rtcp_feedback_mut() = matched_cap_codec.rtcp_feedback().clone();
            consumer_params.codecs.push(codec);
        }
    }
    // Must sanitize the list of matched codecs by removing useless RTX codecs.
    let mut remove_codecs = Vec::new();
    for (idx, codec) in consumer_params.codecs.iter().enumerate() {
        if codec.is_rtx() {
            // Search for the associated media codec.
            let associated_media_codec = consumer_params.codecs.iter().find(|media_codec| {
                match codec.parameters().get("apt") {
                    Some(RtpCodecParametersParametersValue::Number(apt)) => {
                        media_codec.payload_type() as u32 == *apt
                    }
                    _ => false,
                }
            });

            if associated_media_codec.is_some() {
                rtx_supported = true;
            } else {
                remove_codecs.push(idx);
            }
        }
    }
    for idx in remove_codecs.into_iter().rev() {
        consumer_params.codecs.remove(idx);
    }

    // Ensure there is at least one media codec.
    if consumer_params.codecs.is_empty() || consumer_params.codecs[0].is_rtx() {
        return Err(ConsumerRtpParametersError::NoCompatibleMediaCodecs);
    }

    consumer_params.header_extensions = consumable_params
        .header_extensions
        .iter()
        .filter(|ext| {
            caps.header_extensions
                .iter()
                .any(|cap_ext| cap_ext.preferred_id == ext.id && cap_ext.uri == ext.uri)
        })
        .cloned()
        .collect();

    // Reduce codecs' RTCP feedback. Use Transport-CC if available, REMB otherwise.
    if consumer_params
        .header_extensions
        .iter()
        .any(|ext| ext.uri == RtpHeaderExtensionUri::TransportWideCCDraft01)
    {
        for codec in consumer_params.codecs.iter_mut() {
            codec
                .rtcp_feedback_mut()
                .retain(|fb| fb != &RtcpFeedback::GoogRemb);
        }
    } else if consumer_params
        .header_extensions
        .iter()
        .any(|ext| ext.uri == RtpHeaderExtensionUri::AbsSendTime)
    {
        for codec in consumer_params.codecs.iter_mut() {
            codec
                .rtcp_feedback_mut()
                .retain(|fb| fb != &RtcpFeedback::TransportCC);
        }
    } else {
        for codec in consumer_params.codecs.iter_mut() {
            codec
                .rtcp_feedback_mut()
                .retain(|fb| !matches!(fb, RtcpFeedback::GoogRemb | RtcpFeedback::TransportCC));
        }
    }

    let mut consumer_encoding = RtpEncodingParameters {
        ssrc: Some(generate_ssrc()),
        ..RtpEncodingParameters::default()
    };

    if rtx_supported {
        consumer_encoding.rtx = Some(RtpEncodingParametersRtx {
            ssrc: generate_ssrc(),
        });
    }

    // If any of the consumable_params.encodings has scalability_mode, process it
    // (assume all encodings have the same value).
    let mut scalability_mode = consumable_params
        .encodings
        .iter()
        .find_map(|encoding| encoding.scalability_mode.clone());

    // If there is simulast, mangle spatial layers in scalabilityMode.
    if consumable_params.encodings.len() > 1 {
        scalability_mode = Some(format!(
            "S{}T{}",
            consumable_params.encodings.len(),
            scalability_mode
                .as_ref()
                .map(|s| s.parse::<ScalabilityMode>().ok())
                .flatten()
                .unwrap_or_default()
                .temporal_layers
        ));
    }

    consumer_encoding.scalability_mode = scalability_mode;

    // Use the maximum max_bitrate in any encoding and honor it in the Consumer's encoding.
    consumer_encoding.max_bitrate = consumable_params
        .encodings
        .iter()
        .map(|encoding| encoding.max_bitrate)
        .max()
        .flatten();

    // Set a single encoding for the Consumer.
    consumer_params.encodings.push(consumer_encoding);

    // Copy verbatim.
    consumer_params.rtcp = consumable_params.rtcp.clone();

    Ok(consumer_params)
}

/// Generate RTP parameters for a pipe Consumer.
///
/// It keeps all original consumable encodings and removes support for BWE. If
/// enableRtx is false, it also removes RTX and NACK support.
pub(crate) fn get_pipe_consumer_rtp_parameters(
    consumable_params: &RtpParameters,
    enable_rtx: bool,
) -> RtpParameters {
    let mut consumer_params = RtpParameters {
        mid: None,
        codecs: vec![],
        header_extensions: vec![],
        encodings: vec![],
        rtcp: consumable_params.rtcp.clone(),
    };

    for codec in consumable_params.codecs.iter() {
        if !enable_rtx && codec.is_rtx() {
            continue;
        }

        let mut codec = codec.clone();

        codec.rtcp_feedback_mut().retain(|fb| {
            matches!(fb, RtcpFeedback::NackPli | RtcpFeedback::CcmFir)
                || (enable_rtx && fb == &RtcpFeedback::Nack)
        });

        consumer_params.codecs.push(codec);
    }

    // Reduce RTP extensions by disabling transport MID and BWE related ones.
    consumer_params.header_extensions = consumable_params
        .header_extensions
        .iter()
        .filter(|ext| {
            !matches!(
                ext.uri,
                RtpHeaderExtensionUri::MID
                    | RtpHeaderExtensionUri::AbsSendTime
                    | RtpHeaderExtensionUri::TransportWideCCDraft01
            )
        })
        .cloned()
        .collect();

    for ((encoding, ssrc), rtx_ssrc) in consumable_params
        .encodings
        .iter()
        .zip(generate_ssrc()..)
        .zip(generate_ssrc()..)
    {
        consumer_params.encodings.push(RtpEncodingParameters {
            ssrc: Some(ssrc),
            rtx: if enable_rtx {
                Some(RtpEncodingParametersRtx { ssrc: rtx_ssrc })
            } else {
                None
            },
            ..encoding.clone()
        });
    }

    consumer_params
}

struct CodecToMatch<'a> {
    channels: Option<NonZeroU8>,
    clock_rate: NonZeroU32,
    mime_type: MimeType,
    parameters: &'a RtpCodecParametersParameters,
}

impl<'a> From<&'a RtpCodecCapability> for CodecToMatch<'a> {
    fn from(rtp_codec_capability: &'a RtpCodecCapability) -> Self {
        match rtp_codec_capability {
            RtpCodecCapability::Audio {
                mime_type,
                channels,
                clock_rate,
                parameters,
                ..
            } => Self {
                channels: Some(*channels),
                clock_rate: *clock_rate,
                mime_type: MimeType::Audio(*mime_type),
                parameters,
            },
            RtpCodecCapability::Video {
                mime_type,
                clock_rate,
                parameters,
                ..
            } => Self {
                channels: None,
                clock_rate: *clock_rate,
                mime_type: MimeType::Video(*mime_type),
                parameters,
            },
        }
    }
}

impl<'a> From<&'a RtpCodecCapabilityFinalized> for CodecToMatch<'a> {
    fn from(rtp_codec_capability: &'a RtpCodecCapabilityFinalized) -> Self {
        match rtp_codec_capability {
            RtpCodecCapabilityFinalized::Audio {
                mime_type,
                channels,
                clock_rate,
                parameters,
                ..
            } => Self {
                channels: Some(*channels),
                clock_rate: *clock_rate,
                mime_type: MimeType::Audio(*mime_type),
                parameters,
            },
            RtpCodecCapabilityFinalized::Video {
                mime_type,
                clock_rate,
                parameters,
                ..
            } => Self {
                channels: None,
                clock_rate: *clock_rate,
                mime_type: MimeType::Video(*mime_type),
                parameters,
            },
        }
    }
}

impl<'a> From<&'a RtpCodecParameters> for CodecToMatch<'a> {
    fn from(rtp_codec_parameters: &'a RtpCodecParameters) -> Self {
        match rtp_codec_parameters {
            RtpCodecParameters::Audio {
                mime_type,
                channels,
                clock_rate,
                parameters,
                ..
            } => Self {
                channels: Some(*channels),
                clock_rate: *clock_rate,
                mime_type: MimeType::Audio(*mime_type),
                parameters,
            },
            RtpCodecParameters::Video {
                mime_type,
                clock_rate,
                parameters,
                ..
            } => Self {
                channels: None,
                clock_rate: *clock_rate,
                mime_type: MimeType::Video(*mime_type),
                parameters,
            },
        }
    }
}

/// Returns selected `Ok(Some(profile-level-id))` for H264 codec and `Ok(None)` for others
fn match_codecs(
    codec_a: CodecToMatch,
    codec_b: CodecToMatch,
    strict: bool,
) -> Result<Option<String>, ()> {
    if codec_a.mime_type != codec_b.mime_type {
        return Err(());
    }

    if codec_a.channels != codec_b.channels {
        return Err(());
    }

    if codec_a.clock_rate != codec_b.clock_rate {
        return Err(());
    }
    // Per codec special checks.
    match codec_a.mime_type {
        MimeType::Video(MimeTypeVideo::H264) => {
            let packetization_mode_a = codec_a
                .parameters
                .get("packetization-mode")
                .unwrap_or(&RtpCodecParametersParametersValue::Number(0));
            let packetization_mode_b = codec_b
                .parameters
                .get("packetization-mode")
                .unwrap_or(&RtpCodecParametersParametersValue::Number(0));

            if packetization_mode_a != packetization_mode_b {
                return Err(());
            }

            // If strict matching check profile-level-id.
            if strict {
                let profile_level_id_a =
                    codec_a
                        .parameters
                        .get("profile-level-id")
                        .and_then(|p| match p {
                            RtpCodecParametersParametersValue::String(s) => Some(s.as_str()),
                            RtpCodecParametersParametersValue::Number(_) => None,
                        });
                let profile_level_id_b =
                    codec_b
                        .parameters
                        .get("profile-level-id")
                        .and_then(|p| match p {
                            RtpCodecParametersParametersValue::String(s) => Some(s.as_str()),
                            RtpCodecParametersParametersValue::Number(_) => None,
                        });

                let (profile_level_id_a, profile_level_id_b) =
                    match h264_profile_level_id::is_same_profile(
                        profile_level_id_a,
                        profile_level_id_b,
                    ) {
                        Some((profile_level_id_a, profile_level_id_b)) => {
                            (profile_level_id_a, profile_level_id_b)
                        }
                        None => {
                            return Err(());
                        }
                    };

                let selected_profile_level_id =
                    h264_profile_level_id::generate_profile_level_id_for_answer(
                        Some(profile_level_id_a),
                        codec_a
                            .parameters
                            .get("level-asymmetry-allowed")
                            .map(|p| p == &RtpCodecParametersParametersValue::Number(1))
                            .unwrap_or_default(),
                        Some(profile_level_id_b),
                        codec_b
                            .parameters
                            .get("level-asymmetry-allowed")
                            .map(|p| p == &RtpCodecParametersParametersValue::Number(1))
                            .unwrap_or_default(),
                    );

                return match selected_profile_level_id {
                    Ok(selected_profile_level_id) => {
                        Ok(Some(selected_profile_level_id.to_string()))
                    }
                    Err(_) => Err(()),
                };
            }
        }

        MimeType::Video(MimeTypeVideo::VP9) => {
            // If strict matching check profile-id.
            if strict {
                let profile_id_a = codec_a
                    .parameters
                    .get("profile-id")
                    .unwrap_or(&RtpCodecParametersParametersValue::Number(0));
                let profile_id_b = codec_b
                    .parameters
                    .get("profile-id")
                    .unwrap_or(&RtpCodecParametersParametersValue::Number(0));

                if profile_id_a != profile_id_b {
                    return Err(());
                }
            }
        }

        _ => {}
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtp_parameters::{MimeTypeAudio, RtpHeaderExtension};
    use std::iter;

    #[test]
    fn generate_router_rtp_capabilities_succeeds() {
        let media_codecs = vec![
            RtpCodecCapability::Audio {
                mime_type: MimeTypeAudio::Opus,
                preferred_payload_type: None,
                clock_rate: NonZeroU32::new(48000).unwrap(),
                channels: NonZeroU8::new(2).unwrap(),
                parameters: RtpCodecParametersParameters::from([
                    ("useinbandfec", 1u32.into()),
                    ("foo", "bar".into()),
                ]),
                rtcp_feedback: vec![],
            },
            RtpCodecCapability::Video {
                mime_type: MimeTypeVideo::VP8,
                preferred_payload_type: Some(125), // Let's force it.
                clock_rate: NonZeroU32::new(90000).unwrap(),
                parameters: RtpCodecParametersParameters::new(),
                rtcp_feedback: vec![],
            },
            RtpCodecCapability::Video {
                mime_type: MimeTypeVideo::H264,
                preferred_payload_type: None,
                clock_rate: NonZeroU32::new(90000).unwrap(),
                parameters: RtpCodecParametersParameters::from([
                    ("level-asymmetry-allowed", 1u32.into()),
                    ("profile-level-id", "42e01f".into()),
                    ("foo", "bar".into()),
                ]),
                rtcp_feedback: vec![], // Will be ignored.
            },
        ];

        let rtp_capabilities = generate_router_rtp_capabilities(media_codecs)
            .expect("Failed to generate router RTP capabilities");

        assert_eq!(
            rtp_capabilities.codecs,
            vec![
                RtpCodecCapabilityFinalized::Audio {
                    mime_type: MimeTypeAudio::Opus,
                    preferred_payload_type: 100, // 100 is the first available dynamic PT.
                    clock_rate: NonZeroU32::new(48000).unwrap(),
                    channels: NonZeroU8::new(2).unwrap(),
                    parameters: RtpCodecParametersParameters::from([
                        ("useinbandfec", 1u32.into()),
                        ("foo", "bar".into()),
                    ]),
                    rtcp_feedback: vec![RtcpFeedback::TransportCC],
                },
                RtpCodecCapabilityFinalized::Video {
                    mime_type: MimeTypeVideo::VP8,
                    preferred_payload_type: 125,
                    clock_rate: NonZeroU32::new(90000).unwrap(),
                    parameters: RtpCodecParametersParameters::new(),
                    rtcp_feedback: vec![
                        RtcpFeedback::Nack,
                        RtcpFeedback::NackPli,
                        RtcpFeedback::CcmFir,
                        RtcpFeedback::GoogRemb,
                        RtcpFeedback::TransportCC
                    ],
                },
                RtpCodecCapabilityFinalized::Video {
                    mime_type: MimeTypeVideo::RTX,
                    preferred_payload_type: 101, // 101 is the second available dynamic PT.
                    clock_rate: NonZeroU32::new(90000).unwrap(),
                    parameters: RtpCodecParametersParameters::from([("apt", 125u32.into())]),
                    rtcp_feedback: vec![],
                },
                RtpCodecCapabilityFinalized::Video {
                    mime_type: MimeTypeVideo::H264,
                    preferred_payload_type: 102, // 102 is the third available dynamic PT.
                    clock_rate: NonZeroU32::new(90000).unwrap(),
                    parameters: RtpCodecParametersParameters::from([
                        ("packetization-mode", 0u32.into()),
                        ("level-asymmetry-allowed", 1u32.into()),
                        ("profile-level-id", "42e01f".into()),
                        ("foo", "bar".into()),
                    ]),
                    rtcp_feedback: vec![
                        RtcpFeedback::Nack,
                        RtcpFeedback::NackPli,
                        RtcpFeedback::CcmFir,
                        RtcpFeedback::GoogRemb,
                        RtcpFeedback::TransportCC,
                    ],
                },
                RtpCodecCapabilityFinalized::Video {
                    mime_type: MimeTypeVideo::RTX,
                    preferred_payload_type: 103,
                    clock_rate: NonZeroU32::new(90000).unwrap(),
                    parameters: RtpCodecParametersParameters::from([("apt", 102u32.into())]),
                    rtcp_feedback: vec![],
                },
            ]
        );
    }

    #[test]
    fn generate_router_rtp_capabilities_unsupported() {
        assert!(matches!(
            generate_router_rtp_capabilities(vec![RtpCodecCapability::Audio {
                mime_type: MimeTypeAudio::Opus,
                preferred_payload_type: None,
                clock_rate: NonZeroU32::new(48000).unwrap(),
                channels: NonZeroU8::new(1).unwrap(),
                parameters: RtpCodecParametersParameters::new(),
                rtcp_feedback: vec![],
            }]),
            Err(RtpCapabilitiesError::UnsupportedCodec { .. })
        ));

        assert!(matches!(
            generate_router_rtp_capabilities(vec![RtpCodecCapability::Video {
                mime_type: MimeTypeVideo::H264,
                preferred_payload_type: None,
                clock_rate: NonZeroU32::new(90000).unwrap(),
                parameters: RtpCodecParametersParameters::from([(
                    "packetization-mode",
                    5u32.into()
                )]),
                rtcp_feedback: vec![],
            }]),
            Err(RtpCapabilitiesError::UnsupportedCodec { .. })
        ));
    }

    #[test]
    fn generate_router_rtp_capabilities_too_many_codecs() {
        assert!(matches!(
            generate_router_rtp_capabilities(
                iter::repeat(RtpCodecCapability::Audio {
                    mime_type: MimeTypeAudio::Opus,
                    preferred_payload_type: None,
                    clock_rate: NonZeroU32::new(48000).unwrap(),
                    channels: NonZeroU8::new(2).unwrap(),
                    parameters: RtpCodecParametersParameters::new(),
                    rtcp_feedback: vec![],
                })
                .take(100)
                .collect::<Vec<_>>()
            ),
            Err(RtpCapabilitiesError::CannotAllocate)
        ));
    }

    #[test]
    fn get_producer_rtp_parameters_mapping_get_consumable_rtp_parameters_get_consumer_rtp_parameters_get_pipe_consumer_rtp_parameters_succeeds(
    ) {
        let media_codecs = vec![
            RtpCodecCapability::Audio {
                mime_type: MimeTypeAudio::Opus,
                preferred_payload_type: None,
                clock_rate: NonZeroU32::new(48000).unwrap(),
                channels: NonZeroU8::new(2).unwrap(),
                parameters: RtpCodecParametersParameters::from([
                    ("useinbandfec", 1u32.into()),
                    ("foo", "bar".into()),
                ]),
                rtcp_feedback: vec![],
            },
            RtpCodecCapability::Video {
                mime_type: MimeTypeVideo::H264,
                preferred_payload_type: None,
                clock_rate: NonZeroU32::new(90000).unwrap(),
                parameters: RtpCodecParametersParameters::from([
                    ("level-asymmetry-allowed", 1u32.into()),
                    ("packetization-mode", 1u32.into()),
                    ("profile-level-id", "4d0032".into()),
                    ("foo", "lalala".into()),
                ]),
                rtcp_feedback: vec![],
            },
        ];

        let router_rtp_capabilities = generate_router_rtp_capabilities(media_codecs)
            .expect("Failed to generate router RTP capabilities");

        let rtp_parameters = RtpParameters {
            mid: None,
            codecs: vec![
                RtpCodecParameters::Video {
                    mime_type: MimeTypeVideo::H264,
                    payload_type: 111,
                    clock_rate: NonZeroU32::new(90000).unwrap(),
                    parameters: RtpCodecParametersParameters::from([
                        ("foo", 1234u32.into()),
                        ("packetization-mode", 1u32.into()),
                        ("profile-level-id", "4d0032".into()),
                    ]),
                    rtcp_feedback: vec![
                        RtcpFeedback::Nack,
                        RtcpFeedback::NackPli,
                        RtcpFeedback::GoogRemb,
                    ],
                },
                RtpCodecParameters::Video {
                    mime_type: MimeTypeVideo::RTX,
                    payload_type: 112,
                    clock_rate: NonZeroU32::new(90000).unwrap(),
                    parameters: RtpCodecParametersParameters::from([("apt", 111u32.into())]),
                    rtcp_feedback: vec![],
                },
            ],
            header_extensions: vec![
                RtpHeaderExtensionParameters {
                    uri: RtpHeaderExtensionUri::MID,
                    id: 1,
                    encrypt: false,
                },
                RtpHeaderExtensionParameters {
                    uri: RtpHeaderExtensionUri::VideoOrientation,
                    id: 2,
                    encrypt: false,
                },
            ],
            encodings: vec![
                RtpEncodingParameters {
                    ssrc: Some(11111111),
                    rtx: Some(RtpEncodingParametersRtx { ssrc: 11111112 }),
                    scalability_mode: Some("L1T3".to_string()),
                    max_bitrate: Some(111111),
                    ..RtpEncodingParameters::default()
                },
                RtpEncodingParameters {
                    ssrc: Some(21111111),
                    rtx: Some(RtpEncodingParametersRtx { ssrc: 21111112 }),
                    scalability_mode: Some("L1T3".to_string()),
                    max_bitrate: Some(222222),
                    ..RtpEncodingParameters::default()
                },
                RtpEncodingParameters {
                    rid: Some("high".to_string()),
                    scalability_mode: Some("L1T3".to_string()),
                    max_bitrate: Some(333333),
                    ..RtpEncodingParameters::default()
                },
            ],
            rtcp: RtcpParameters {
                cname: Some("qwerty1234".to_string()),
                ..RtcpParameters::default()
            },
        };

        let rtp_mapping =
            get_producer_rtp_parameters_mapping(&rtp_parameters, &router_rtp_capabilities)
                .expect("Failed to get producer RTP parameters mapping");

        assert_eq!(
            rtp_mapping.codecs,
            vec![
                RtpMappingCodec {
                    payload_type: 111,
                    mapped_payload_type: 101
                },
                RtpMappingCodec {
                    payload_type: 112,
                    mapped_payload_type: 102
                },
            ]
        );

        assert_eq!(rtp_mapping.encodings.get(0).unwrap().ssrc, Some(11111111));
        assert_eq!(rtp_mapping.encodings.get(0).unwrap().rid, None);
        assert_eq!(rtp_mapping.encodings.get(1).unwrap().ssrc, Some(21111111));
        assert_eq!(rtp_mapping.encodings.get(1).unwrap().rid, None);
        assert_eq!(rtp_mapping.encodings.get(2).unwrap().ssrc, None);
        assert_eq!(
            rtp_mapping.encodings.get(2).unwrap().rid,
            Some("high".to_string())
        );

        let consumable_rtp_parameters = get_consumable_rtp_parameters(
            MediaKind::Video,
            &rtp_parameters,
            &router_rtp_capabilities,
            &rtp_mapping,
        );

        assert_eq!(
            consumable_rtp_parameters.codecs,
            vec![
                RtpCodecParameters::Video {
                    mime_type: MimeTypeVideo::H264,
                    payload_type: 101,
                    clock_rate: NonZeroU32::new(90000).unwrap(),
                    parameters: RtpCodecParametersParameters::from([
                        ("foo", 1234u32.into()),
                        ("packetization-mode", 1u32.into()),
                        ("profile-level-id", "4d0032".into()),
                    ]),
                    rtcp_feedback: vec![
                        RtcpFeedback::Nack,
                        RtcpFeedback::NackPli,
                        RtcpFeedback::CcmFir,
                        RtcpFeedback::GoogRemb,
                        RtcpFeedback::TransportCC,
                    ],
                },
                RtpCodecParameters::Video {
                    mime_type: MimeTypeVideo::RTX,
                    payload_type: 102,
                    clock_rate: NonZeroU32::new(90000).unwrap(),
                    parameters: RtpCodecParametersParameters::from([("apt", 101u32.into())]),
                    rtcp_feedback: vec![],
                },
            ]
        );

        assert_eq!(
            consumable_rtp_parameters.encodings.get(0).unwrap().ssrc,
            Some(rtp_mapping.encodings.get(0).unwrap().mapped_ssrc),
        );
        assert_eq!(
            consumable_rtp_parameters
                .encodings
                .get(0)
                .unwrap()
                .max_bitrate,
            Some(111111),
        );
        assert_eq!(
            consumable_rtp_parameters
                .encodings
                .get(0)
                .unwrap()
                .scalability_mode,
            Some("L1T3".to_string()),
        );
        assert_eq!(
            consumable_rtp_parameters.encodings.get(1).unwrap().ssrc,
            Some(rtp_mapping.encodings.get(1).unwrap().mapped_ssrc),
        );
        assert_eq!(
            consumable_rtp_parameters
                .encodings
                .get(1)
                .unwrap()
                .max_bitrate,
            Some(222222),
        );
        assert_eq!(
            consumable_rtp_parameters
                .encodings
                .get(1)
                .unwrap()
                .scalability_mode,
            Some("L1T3".to_string()),
        );
        assert_eq!(
            consumable_rtp_parameters.encodings.get(2).unwrap().ssrc,
            Some(rtp_mapping.encodings.get(2).unwrap().mapped_ssrc),
        );
        assert_eq!(
            consumable_rtp_parameters
                .encodings
                .get(2)
                .unwrap()
                .max_bitrate,
            Some(333333),
        );
        assert_eq!(
            consumable_rtp_parameters
                .encodings
                .get(2)
                .unwrap()
                .scalability_mode,
            Some("L1T3".to_string()),
        );

        assert_eq!(
            consumable_rtp_parameters.rtcp,
            RtcpParameters {
                cname: rtp_parameters.rtcp.cname.clone(),
                reduced_size: true,
                mux: Some(true),
            }
        );

        let remote_rtp_capabilities = RtpCapabilities {
            codecs: vec![
                RtpCodecCapability::Audio {
                    mime_type: MimeTypeAudio::Opus,
                    preferred_payload_type: Some(100),
                    clock_rate: NonZeroU32::new(48000).unwrap(),
                    channels: NonZeroU8::new(2).unwrap(),
                    parameters: RtpCodecParametersParameters::new(),
                    rtcp_feedback: vec![],
                },
                RtpCodecCapability::Video {
                    mime_type: MimeTypeVideo::H264,
                    preferred_payload_type: Some(101),
                    clock_rate: NonZeroU32::new(90000).unwrap(),
                    parameters: RtpCodecParametersParameters::from([
                        ("packetization-mode", 1u32.into()),
                        ("profile-level-id", "4d0032".into()),
                        ("baz", "LOLOLO".into()),
                    ]),
                    rtcp_feedback: vec![
                        RtcpFeedback::Nack,
                        RtcpFeedback::NackPli,
                        RtcpFeedback::Unsupported,
                    ],
                },
                RtpCodecCapability::Video {
                    mime_type: MimeTypeVideo::RTX,
                    preferred_payload_type: Some(102),
                    clock_rate: NonZeroU32::new(90000).unwrap(),
                    parameters: RtpCodecParametersParameters::from([("apt", 101u32.into())]),
                    rtcp_feedback: vec![],
                },
            ],
            header_extensions: vec![
                RtpHeaderExtension {
                    kind: Some(MediaKind::Audio),
                    uri: RtpHeaderExtensionUri::MID,
                    preferred_id: 1,
                    preferred_encrypt: false,
                    direction: RtpHeaderExtensionDirection::SendRecv,
                },
                RtpHeaderExtension {
                    kind: Some(MediaKind::Video),
                    uri: RtpHeaderExtensionUri::MID,
                    preferred_id: 1,
                    preferred_encrypt: false,
                    direction: RtpHeaderExtensionDirection::SendRecv,
                },
                RtpHeaderExtension {
                    kind: Some(MediaKind::Video),
                    uri: RtpHeaderExtensionUri::RtpStreamId,
                    preferred_id: 2,
                    preferred_encrypt: false,
                    direction: RtpHeaderExtensionDirection::SendRecv,
                },
                RtpHeaderExtension {
                    kind: Some(MediaKind::Audio),
                    uri: RtpHeaderExtensionUri::AudioLevel,
                    preferred_id: 8,
                    preferred_encrypt: false,
                    direction: RtpHeaderExtensionDirection::SendRecv,
                },
                RtpHeaderExtension {
                    kind: Some(MediaKind::Video),
                    uri: RtpHeaderExtensionUri::VideoOrientation,
                    preferred_id: 11,
                    preferred_encrypt: false,
                    direction: RtpHeaderExtensionDirection::SendRecv,
                },
                RtpHeaderExtension {
                    kind: Some(MediaKind::Video),
                    uri: RtpHeaderExtensionUri::TimeOffset,
                    preferred_id: 12,
                    preferred_encrypt: false,
                    direction: RtpHeaderExtensionDirection::SendRecv,
                },
            ],
            fec_mechanisms: vec![],
        };

        let consumer_rtp_parameters =
            get_consumer_rtp_parameters(&consumable_rtp_parameters, remote_rtp_capabilities)
                .expect("Failed to get consumer RTP parameters");

        assert_eq!(
            consumer_rtp_parameters.codecs,
            vec![
                RtpCodecParameters::Video {
                    mime_type: MimeTypeVideo::H264,
                    payload_type: 101,
                    clock_rate: NonZeroU32::new(90000).unwrap(),
                    parameters: RtpCodecParametersParameters::from([
                        ("foo", 1234u32.into()),
                        ("packetization-mode", 1u32.into()),
                        ("profile-level-id", "4d0032".into()),
                    ]),
                    rtcp_feedback: vec![
                        RtcpFeedback::Nack,
                        RtcpFeedback::NackPli,
                        RtcpFeedback::Unsupported,
                    ],
                },
                RtpCodecParameters::Video {
                    mime_type: MimeTypeVideo::RTX,
                    payload_type: 102,
                    clock_rate: NonZeroU32::new(90000).unwrap(),
                    parameters: RtpCodecParametersParameters::from([("apt", 101u32.into())]),
                    rtcp_feedback: vec![],
                },
            ]
        );

        assert_eq!(consumer_rtp_parameters.encodings.len(), 1);
        assert!(consumer_rtp_parameters
            .encodings
            .get(0)
            .unwrap()
            .ssrc
            .is_some());
        assert!(consumer_rtp_parameters
            .encodings
            .get(0)
            .unwrap()
            .rtx
            .is_some());
        assert_eq!(
            consumer_rtp_parameters
                .encodings
                .get(0)
                .unwrap()
                .scalability_mode,
            Some("S3T3".to_string()),
        );
        assert_eq!(
            consumer_rtp_parameters
                .encodings
                .get(0)
                .unwrap()
                .max_bitrate,
            Some(333333),
        );

        assert_eq!(
            consumer_rtp_parameters.header_extensions,
            vec![
                RtpHeaderExtensionParameters {
                    uri: RtpHeaderExtensionUri::MID,
                    id: 1,
                    encrypt: false,
                },
                RtpHeaderExtensionParameters {
                    uri: RtpHeaderExtensionUri::VideoOrientation,
                    id: 11,
                    encrypt: false,
                },
                RtpHeaderExtensionParameters {
                    uri: RtpHeaderExtensionUri::TimeOffset,
                    id: 12,
                    encrypt: false,
                },
            ],
        );

        assert_eq!(
            consumer_rtp_parameters.rtcp,
            RtcpParameters {
                cname: rtp_parameters.rtcp.cname.clone(),
                reduced_size: true,
                mux: Some(true),
            },
        );

        let pipe_consumer_rtp_parameters =
            get_pipe_consumer_rtp_parameters(&consumable_rtp_parameters, false);

        assert_eq!(
            pipe_consumer_rtp_parameters.codecs,
            vec![RtpCodecParameters::Video {
                mime_type: MimeTypeVideo::H264,
                payload_type: 101,
                clock_rate: NonZeroU32::new(90000).unwrap(),
                parameters: RtpCodecParametersParameters::from([
                    ("foo", 1234u32.into()),
                    ("packetization-mode", 1u32.into()),
                    ("profile-level-id", "4d0032".into()),
                ]),
                rtcp_feedback: vec![RtcpFeedback::NackPli, RtcpFeedback::CcmFir],
            }],
        );

        assert_eq!(pipe_consumer_rtp_parameters.encodings.len(), 3);
        assert!(pipe_consumer_rtp_parameters
            .encodings
            .get(0)
            .unwrap()
            .ssrc
            .is_some());
        assert!(pipe_consumer_rtp_parameters
            .encodings
            .get(0)
            .unwrap()
            .rtx
            .is_none());
        assert!(pipe_consumer_rtp_parameters
            .encodings
            .get(0)
            .unwrap()
            .max_bitrate
            .is_some());
        assert_eq!(
            pipe_consumer_rtp_parameters
                .encodings
                .get(0)
                .unwrap()
                .scalability_mode,
            Some("L1T3".to_string()),
        );
        assert!(pipe_consumer_rtp_parameters
            .encodings
            .get(1)
            .unwrap()
            .ssrc
            .is_some());
        assert!(pipe_consumer_rtp_parameters
            .encodings
            .get(1)
            .unwrap()
            .rtx
            .is_none());
        assert!(pipe_consumer_rtp_parameters
            .encodings
            .get(1)
            .unwrap()
            .max_bitrate
            .is_some());
        assert_eq!(
            pipe_consumer_rtp_parameters
                .encodings
                .get(1)
                .unwrap()
                .scalability_mode,
            Some("L1T3".to_string()),
        );
        assert!(pipe_consumer_rtp_parameters
            .encodings
            .get(2)
            .unwrap()
            .ssrc
            .is_some());
        assert!(pipe_consumer_rtp_parameters
            .encodings
            .get(2)
            .unwrap()
            .rtx
            .is_none());
        assert!(pipe_consumer_rtp_parameters
            .encodings
            .get(2)
            .unwrap()
            .max_bitrate
            .is_some());
        assert_eq!(
            pipe_consumer_rtp_parameters
                .encodings
                .get(2)
                .unwrap()
                .scalability_mode,
            Some("L1T3".to_string()),
        );

        assert_eq!(
            pipe_consumer_rtp_parameters.rtcp,
            RtcpParameters {
                cname: rtp_parameters.rtcp.cname.clone(),
                reduced_size: true,
                mux: Some(true),
            },
        );
    }

    #[test]
    fn get_producer_rtp_parameters_mapping_unsupported() {
        let media_codecs = vec![
            RtpCodecCapability::Audio {
                mime_type: MimeTypeAudio::Opus,
                preferred_payload_type: None,
                clock_rate: NonZeroU32::new(48000).unwrap(),
                channels: NonZeroU8::new(2).unwrap(),
                parameters: RtpCodecParametersParameters::default(),
                rtcp_feedback: vec![],
            },
            RtpCodecCapability::Video {
                mime_type: MimeTypeVideo::H264,
                preferred_payload_type: None,
                clock_rate: NonZeroU32::new(90000).unwrap(),
                parameters: RtpCodecParametersParameters::from([
                    ("packetization-mode", 1u32.into()),
                    ("profile-level-id", "640032".into()),
                ]),
                rtcp_feedback: vec![],
            },
        ];

        let router_rtp_capabilities = generate_router_rtp_capabilities(media_codecs)
            .expect("Failed to generate router RTP capabilities");

        let rtp_parameters = RtpParameters {
            mid: None,
            codecs: vec![RtpCodecParameters::Video {
                mime_type: MimeTypeVideo::VP8,
                payload_type: 120,
                clock_rate: NonZeroU32::new(90000).unwrap(),
                parameters: RtpCodecParametersParameters::default(),
                rtcp_feedback: vec![RtcpFeedback::Nack, RtcpFeedback::Unsupported],
            }],
            header_extensions: vec![],
            encodings: vec![RtpEncodingParameters {
                ssrc: Some(11111111),
                ..RtpEncodingParameters::default()
            }],
            rtcp: RtcpParameters {
                cname: Some("qwerty1234".to_string()),
                ..RtcpParameters::default()
            },
        };

        assert!(matches!(
            get_producer_rtp_parameters_mapping(&rtp_parameters, &router_rtp_capabilities),
            Err(RtpParametersMappingError::UnsupportedCodec { .. }),
        ));
    }
}
