pub(crate) fn detect_text_encoding(bytes: &[u8]) -> &'static encoding_rs::Encoding {
    let detection =
        compact_enc_det::detect_encoding(bytes, compact_enc_det::DetectHints::default());
    let mut encoding = encoding_rs::Encoding::for_label(detection.mime_name.as_bytes())
        .unwrap_or_else(|| {
            encoding_from_ced(detection.encoding).unwrap_or(encoding_rs::WINDOWS_1252)
        });

    // CED can report GB2312/EUC-CN for GBK-compatible files.
    if matches!(
        detection.encoding,
        compact_enc_det::Encoding::CHINESE_GB
            | compact_enc_det::Encoding::CHINESE_EUC_CN
            | compact_enc_det::Encoding::GBK
    ) {
        encoding = encoding_rs::GBK;
    }
    encoding
}

fn encoding_from_ced(
    encoding: compact_enc_det::Encoding,
) -> Option<&'static encoding_rs::Encoding> {
    match encoding {
        compact_enc_det::Encoding::ASCII_7BIT
        | compact_enc_det::Encoding::UTF8
        | compact_enc_det::Encoding::UTF8UTF8 => Some(encoding_rs::UTF_8),
        compact_enc_det::Encoding::CHINESE_GB
        | compact_enc_det::Encoding::CHINESE_EUC_CN
        | compact_enc_det::Encoding::GBK => Some(encoding_rs::GBK),
        compact_enc_det::Encoding::GB18030 => Some(encoding_rs::GB18030),
        compact_enc_det::Encoding::CHINESE_BIG5
        | compact_enc_det::Encoding::CHINESE_BIG5_CP950
        | compact_enc_det::Encoding::BIG5_HKSCS => Some(encoding_rs::BIG5),
        compact_enc_det::Encoding::JAPANESE_EUC_JP => Some(encoding_rs::EUC_JP),
        compact_enc_det::Encoding::JAPANESE_SHIFT_JIS
        | compact_enc_det::Encoding::JAPANESE_CP932
        | compact_enc_det::Encoding::KDDI_SHIFT_JIS
        | compact_enc_det::Encoding::DOCOMO_SHIFT_JIS
        | compact_enc_det::Encoding::SOFTBANK_SHIFT_JIS => Some(encoding_rs::SHIFT_JIS),
        compact_enc_det::Encoding::JAPANESE_JIS
        | compact_enc_det::Encoding::KDDI_ISO_2022_JP
        | compact_enc_det::Encoding::SOFTBANK_ISO_2022_JP => Some(encoding_rs::ISO_2022_JP),
        compact_enc_det::Encoding::KOREAN_EUC_KR => Some(encoding_rs::EUC_KR),
        compact_enc_det::Encoding::ISO_8859_2 => Some(encoding_rs::ISO_8859_2),
        compact_enc_det::Encoding::ISO_8859_3 => Some(encoding_rs::ISO_8859_3),
        compact_enc_det::Encoding::ISO_8859_4 => Some(encoding_rs::ISO_8859_4),
        compact_enc_det::Encoding::ISO_8859_5 => Some(encoding_rs::ISO_8859_5),
        compact_enc_det::Encoding::ISO_8859_6 => Some(encoding_rs::ISO_8859_6),
        compact_enc_det::Encoding::ISO_8859_7 => Some(encoding_rs::ISO_8859_7),
        compact_enc_det::Encoding::ISO_8859_8 | compact_enc_det::Encoding::HEBREW_VISUAL => {
            Some(encoding_rs::ISO_8859_8)
        }
        compact_enc_det::Encoding::ISO_8859_8_I => Some(encoding_rs::ISO_8859_8_I),
        compact_enc_det::Encoding::ISO_8859_10 => Some(encoding_rs::ISO_8859_10),
        compact_enc_det::Encoding::ISO_8859_13 => Some(encoding_rs::ISO_8859_13),
        compact_enc_det::Encoding::ISO_8859_15 => Some(encoding_rs::ISO_8859_15),
        compact_enc_det::Encoding::MSFT_CP874 | compact_enc_det::Encoding::ISO_8859_11 => {
            Some(encoding_rs::WINDOWS_874)
        }
        compact_enc_det::Encoding::MSFT_CP1250 => Some(encoding_rs::WINDOWS_1250),
        compact_enc_det::Encoding::RUSSIAN_CP1251 => Some(encoding_rs::WINDOWS_1251),
        compact_enc_det::Encoding::MSFT_CP1252 | compact_enc_det::Encoding::ISO_8859_1 => {
            Some(encoding_rs::WINDOWS_1252)
        }
        compact_enc_det::Encoding::MSFT_CP1253 => Some(encoding_rs::WINDOWS_1253),
        compact_enc_det::Encoding::MSFT_CP1254 | compact_enc_det::Encoding::ISO_8859_9 => {
            Some(encoding_rs::WINDOWS_1254)
        }
        compact_enc_det::Encoding::MSFT_CP1255 => Some(encoding_rs::WINDOWS_1255),
        compact_enc_det::Encoding::MSFT_CP1256 => Some(encoding_rs::WINDOWS_1256),
        compact_enc_det::Encoding::MSFT_CP1257 => Some(encoding_rs::WINDOWS_1257),
        compact_enc_det::Encoding::RUSSIAN_KOI8_R | compact_enc_det::Encoding::RUSSIAN_KOI8_RU => {
            Some(encoding_rs::KOI8_R)
        }
        compact_enc_det::Encoding::RUSSIAN_CP866 => Some(encoding_rs::IBM866),
        compact_enc_det::Encoding::MACINTOSH_ROMAN => Some(encoding_rs::MACINTOSH),
        compact_enc_det::Encoding::UTF16BE => Some(encoding_rs::UTF_16BE),
        compact_enc_det::Encoding::UTF16LE | compact_enc_det::Encoding::UNICODE => {
            Some(encoding_rs::UTF_16LE)
        }
        _ => None,
    }
}
