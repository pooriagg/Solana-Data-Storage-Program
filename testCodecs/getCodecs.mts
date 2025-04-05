import {
    getAddressDecoder
} from "@solana/kit";
import {
    getStructEncoder,
    getU8Encoder,
    fixEncoderSize,
    getUtf8Encoder,
    getArrayEncoder,
    transformEncoder,
    getStructDecoder,
    fixDecoderSize,
    getUtf8Decoder,
    getI64Decoder,
    getU8Decoder,
    getBooleanDecoder,
    getArrayDecoder,
    getU16Decoder
} from "@solana/codecs";


export const getCreateDataStorageAccountInstructionDataEncoder = (data_size: number) => {
    const ix_create_new_dsa = getStructEncoder(
        [
            [ "discriminator", getU8Encoder() ],
            [ "label", fixEncoderSize(getUtf8Encoder(), 30) ],
            [ "data", getArrayEncoder(getU8Encoder(), { size: data_size }) ]
        ]
    );

    return transformEncoder(
        ix_create_new_dsa,
        // @ts-ignore
        value => ({ ...value, discriminator: 0 })
    );
}

export const getEditDataStorageAccountInstructionDataEncoder = (data_size: number) => {
    const ix_edit_dsa = getStructEncoder(
        [
            [ "discriminator", getU8Encoder() ],
            [ "newData", getArrayEncoder(getU8Encoder(), { size: data_size }) ]
        ]
    );

    return transformEncoder(
        ix_edit_dsa,
        // @ts-ignore
        value => ({ ...value, discriminator: 1 })
    );
};

export const getCloseDataStorageAccountInstructionDataEncoder = () => {
    const ix_close_dsa = getStructEncoder(
        [
            [ "discriminator", getU8Encoder() ]
        ]
    );

    return transformEncoder(
        ix_close_dsa,
        // @ts-ignore
        value => ({ ...value, discriminator: 2 })
    );
};

export const getDataStorageAccountDecoder = () => {
    return getStructDecoder(
        [
            [ "authority", getAddressDecoder() ],
            [ "label", fixDecoderSize(getUtf8Decoder(), 30) ],
            [ "lastUpdated", getI64Decoder() ],
            [ "canonicalBump", getU8Decoder() ],
            [ "isInitialized", getBooleanDecoder() ],
            [ "data", getArrayDecoder(getU8Decoder(), { size: getU16Decoder() }) ]
        ]
    );
};