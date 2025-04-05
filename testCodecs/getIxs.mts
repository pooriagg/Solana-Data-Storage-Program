import {
    AccountRole,
    Address,
    IInstruction
} from "@solana/kit";

import {
    getCreateDataStorageAccountInstructionDataEncoder,
    getEditDataStorageAccountInstructionDataEncoder,
    getCloseDataStorageAccountInstructionDataEncoder
} from "./getCodecs.mjs";


const SYSTEM_PROGRAM_ID = "11111111111111111111111111111111" as Address;
const DATA_STORAGE_PROGRAM_ID = "DSAgnFyNE53P9m5vz9ALojPQwbtaPjwzPa61ZN1oe7mG" as Address;

export const getCreateDataStorageAccountInstruction = (
    params: {
        new_data_storage_pda: Address,
        data_storage_authority: Address,
        funding_account: Address,
        label: string,
        data: Uint8Array
    }
): IInstruction => {
    const {
        new_data_storage_pda,
        data_storage_authority,
        funding_account,
        label,
        data
    } = params;

    return {
        programAddress: DATA_STORAGE_PROGRAM_ID,
        accounts: [
            {
                address: new_data_storage_pda,
                role: AccountRole.WRITABLE
            },
            {
                address: data_storage_authority,
                role: AccountRole.READONLY_SIGNER
            },
            {
                address: funding_account,
                role: AccountRole.WRITABLE_SIGNER
            },
            {
                address: SYSTEM_PROGRAM_ID,
                role: AccountRole.READONLY
            }
        ],
        data: getCreateDataStorageAccountInstructionDataEncoder(data.length).encode(
            {
                label,
                data
            }
        )
    };
};

export const getEditDataStorageAccountWithNewDataLenIsEqualToOldDataLenInstruction = (
    params: {
        data_storage_pda: Address,
        data_storage_authority: Address,
        new_data: Uint8Array
    }
): IInstruction => {
    const {
        data_storage_authority,
        data_storage_pda,
        new_data
    } = params;

    return {
        programAddress: DATA_STORAGE_PROGRAM_ID,
        accounts: [
            {
                address: data_storage_pda,
                role: AccountRole.WRITABLE
            },
            {
                address: data_storage_authority,
                role: AccountRole.READONLY_SIGNER
            }
        ],
        data: getEditDataStorageAccountInstructionDataEncoder(new_data.length).encode(
            {
                newData: new_data
            }
        )
    };
};

export const getEditDataStorageAccountWithNewDataLenLessIsThanOldDataLenInstruction = (
    params: {
        data_storage_pda: Address,
        data_storage_authority: Address,
        rent_receiver_account: Address,
        new_data: Uint8Array
    }
): IInstruction => {
    const {
        data_storage_authority,
        data_storage_pda,
        rent_receiver_account,
        new_data
    } = params;

    return {
        programAddress: DATA_STORAGE_PROGRAM_ID,
        accounts: [
            {
                address: data_storage_pda,
                role: AccountRole.WRITABLE
            },
            {
                address: data_storage_authority,
                role: AccountRole.READONLY_SIGNER
            },
            {
                address: rent_receiver_account,
                role: AccountRole.WRITABLE
            }
        ],
        data: getEditDataStorageAccountInstructionDataEncoder(new_data.length).encode(
            {
                newData: new_data
            }
        )
    };
};

export const getEditDataStorageAccountWithNewDataLenIsBiggerThanOldDataLenInstruction = (
    params: {
        data_storage_pda: Address,
        data_storage_authority: Address,
        funding_account: Address,
        new_data: Uint8Array
    }
): IInstruction => {
    const {
        data_storage_authority,
        data_storage_pda,
        funding_account,
        new_data
    } = params;

    return {
        programAddress: DATA_STORAGE_PROGRAM_ID,
        accounts: [
            {
                address: data_storage_pda,
                role: AccountRole.WRITABLE
            },
            {
                address: data_storage_authority,
                role: AccountRole.READONLY_SIGNER
            },
            {
                address: funding_account,
                role: AccountRole.WRITABLE_SIGNER
            },
            {
                address: SYSTEM_PROGRAM_ID,
                role: AccountRole.READONLY
            }
        ],
        data: getEditDataStorageAccountInstructionDataEncoder(new_data.length).encode(
            {
                newData: new_data
            }
        )
    };
}

export const getCloseDataStorageAccountInstruction = (
    params: {
        data_storage_pda: Address,
        data_storage_authority: Address,
        rent_exempt_receiver_account: Address
    }
): IInstruction => {
    const {
        data_storage_authority,
        data_storage_pda,
        rent_exempt_receiver_account
    } = params;

    return {
        programAddress: DATA_STORAGE_PROGRAM_ID,
        accounts: [
            {
                address: data_storage_pda,
                role: AccountRole.WRITABLE
            },
            {
                address: data_storage_authority,
                role: AccountRole.READONLY_SIGNER
            },
            {
                address: rent_exempt_receiver_account,
                role: AccountRole.WRITABLE
            },
        ],
        data: getCloseDataStorageAccountInstructionDataEncoder().encode(null)
    };
};