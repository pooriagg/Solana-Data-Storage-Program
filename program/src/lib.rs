use {
    solana_program::{
        account_info::{
            AccountInfo,
            next_account_info
        },

        program_error::ProgramError,

        program::{
            invoke,
            invoke_signed
        },

        pubkey::Pubkey,

        entrypoint_no_alloc,

        entrypoint::ProgramResult,

        system_program::{
            check_id as check_system_program_id,
            ID as SYSTEM_PROGRAM_ID
        },

        system_instruction::{
            transfer as transfer_lamports,
            assign as assign_new_owner,
            allocate as allocate_memory
        },

        sysvar::{
            Sysvar,
            clock::Clock
        },

        program_memory::{
            sol_memcmp,
            sol_memcpy
        },

        log::{
            sol_log,
            msg
        }
    },

    thiserror::Error,

    std::mem::size_of,

    helper::*
};


entrypoint_no_alloc!(process_instruction);

// Instructions discriminator
pub const CREATE_NEW_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR: u8 = 0;
pub const EDIT_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR: u8 = 1;
pub const CLOSE_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR: u8 = 2;

// Constants
pub const MAX_LABEL_LENGTH: usize = 30;

// event emitter
macro_rules! emit {
    ($event: ident) => {
        msg!("{:?}", $event);
    };
}


//? program's instructions

/// NOTE: r readonly, w writable, s signer, x program

// "CREATE_NEW_DATA_STORAGE_ACCOUNT" ix
//  > instruction-data :  
//      0. 'u8' as instruction's discriminator
//      1. '[u8; 30]' as data-account's label (utf-8)
//      2. '[u8; n]' as data-account's data-field
//  > instruction-accounts :
//      0. new data storage account pda - rw
//      1. data-storage account's authority - If authority is SYSTEM_PROGRAM "r" otherwise "rs"
//      2. funding account - rws
//      3. system program account - rx
//  NOTE: We can make a data-storage account immutable with passing system-program-account as authority account.

// "EDIT_DATA_STORAGE_ACCOUNT" ix
// > instruction-data :
//     0. 'u8' as instruction's discriminator
//     1. '[u8; n]' as new data-field
// > instruction-accounts :
//     > if new_data_length == old_data_length :
//          0. data-storage account pda - rw
//          1. data-storage authority account - rs
//     > if new_data_length < old_data_length :
//          0. data-storage account pda - rw
//          1. data-storage authority account - rs
//          2. rent-receiver account info - rw
//     > if new_data_length > old_data_length :
//          0. data-storage account pda - rw
//          1. data-storage authority account - rs
//          2. funding account - rws
//          3. system program account - rx            

// "CLOSE_DATA_STORAGE_ACCOUNT" ix
// > instruction-data :
//      0. 'u8' as instruction's discriminator
// > instruction-accounts :
//      0. data-storage account pda - rw
//      1. data-storage authority account - rs
//      2. rent-exempt receiver account - rw

//? program's instructions


//? program's data account
//      0. 'Pubkey ([u8; 32])' as data-account's owner (..32)
//      1. '[u8; 30]' as data-account's label (utf-8) (32..62)
//      2. 'i64' as last-updated (62..70)
//      3. 'u8' as canonical_bump (70)
//      4. 'bool' as is-initialized (71)
//      5. 'u16' as data-account's data-field length (72..74)
//      6. '[u8; n]' as data-account's data-field (74..)

/// NOTE
/// - Authority account can be a zero-account (system-program-id), to make the data storage account immutable
/// - When initializing a new account 'last-updated' will be '0'
//? program's data account


//? data storage account PDA's seeds
//      0. "data_storage_account"
//      1. authority's Pubkey
//      2. account's label
//? data storage account PDA's seeds

//? data storage account == dsa
///! NOTE: Do not send lamports directly to the data_storage PDAs !
pub fn process_instruction(
    program_id: &Pubkey,
    accounts_info: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult {
    let (
        ix_discriminator,
        ix_data
    ) = instruction_data.split_first().ok_or(ProgramError::InvalidInstructionData)?;

    let accounts_info = &mut accounts_info.iter();

    match *ix_discriminator {
        CREATE_NEW_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR => {
            sol_log("⚙️ Instruction: CreateNewDataStorageAccount");

            let new_data_storage_pda_account_info = next_account_info(accounts_info)?;
            let authority_account_info =  next_account_info(accounts_info)?; 
            let funding_account_info = next_account_info(accounts_info)?;
            let system_program_account_info = next_account_info(accounts_info)?;

            check_system_program_account(system_program_account_info.key)?;

            if authority_account_info.key != &SYSTEM_PROGRAM_ID {
                check_account_is_signer(authority_account_info)?;
                sol_log("Mutable");
            } else {
                sol_log("Immutable");
            };

            // validate instruction-data
            if ix_data.len() < MAX_LABEL_LENGTH {
                return Err(
                    ProgramError::Custom(
                        DataStorageError::InvalidData as u32
                    )
                );
            };

            // deserialize instruction's data
            let (
                account_label,
                account_data
            ) = ix_data.split_at(30);

            // validate label
            if let Err(_) = String::from_utf8(account_label.to_vec()) {
                return Err(
                    ProgramError::Custom(
                        DataStorageError::InvalidLabel as u32
                    )
                );
            };

            // get pda's bump and validate the pda's pubkey
            let (
                dsa_address,
                dsa_bump
            ) = Pubkey::try_find_program_address(
                &[
                    b"data_storage_account",
                    authority_account_info.key.as_ref(),
                    account_label
                ],
                program_id
            ).ok_or::<ProgramError>(ProgramError::Custom(DataStorageError::FailedToFindProgramAddress as u32))?;
            if &dsa_address != new_data_storage_pda_account_info.key {
                return Err(
                    ProgramError::InvalidSeeds
                );
            };

            // create the account
            let account_size = size_of::<Pubkey>() +
                size_of::<[u8; 30]>() +
                size_of::<i64>() +
                size_of::<u8>() +
                size_of::<bool>() +
                size_of::<u16>() +
                account_data.len();

            let seeds: &[&[u8]] = &[
                b"data_storage_account",
                authority_account_info.key.as_ref(),
                account_label,
                &[ dsa_bump ]
            ];

            create_pda_account(
                new_data_storage_pda_account_info,
                funding_account_info,
                account_size,
                program_id,
                seeds
            )?;
            sol_log("New data storage account created.");

            // initialize the account
            // 1. set account-authority
            sol_memcpy(
                new_data_storage_pda_account_info
                    .data
                    .try_borrow_mut()
                    .unwrap()
                    .get_mut(..32)
                    .unwrap(),
                authority_account_info.key.as_ref(),
                size_of::<Pubkey>()
            );
            // 2. set account-label
            sol_memcpy(
                new_data_storage_pda_account_info
                    .data
                    .try_borrow_mut()
                    .unwrap()
                    .get_mut(32..62)
                    .unwrap(),
                    account_label,
                size_of::<[u8; 30]>()
            );
            // 3. skip 'last-updated'
            // 4. set account-bump
            let mut das_data = new_data_storage_pda_account_info
                .data
                .try_borrow_mut()
                .unwrap();
            *das_data
                .get_mut(70)
                .unwrap() = dsa_bump;
            // 5. set is_initialized flag
            *das_data
                .get_mut(71)
                .unwrap() = true as u8;

            drop(das_data);

            // 6. set account-data length and data
            let account_data_len = (account_data.len() as u16).to_le_bytes();
            // 1. set length
            sol_memcpy(
                new_data_storage_pda_account_info
                    .data
                    .try_borrow_mut()
                    .unwrap()
                    .get_mut(72..74)
                    .unwrap(),
                    &account_data_len,
                size_of::<u16>()
            );
            if account_data.len() > 0 {
                // 2. set data
                sol_memcpy(
                    new_data_storage_pda_account_info
                        .data
                        .try_borrow_mut()
                        .unwrap()
                        .get_mut(74..)
                        .unwrap(),
                    account_data,
                    account_data.len()
                );
            };

            let event = Events::NewDataStorageAccountCreated {
                data_storage_account: *new_data_storage_pda_account_info.key,
                authority_account: *authority_account_info.key,
                account_label: account_label
                    .try_into()
                    .unwrap()
            };
            emit!(event);

            sol_log("New data storage account has been initialized successfully. ✅");
        },

        EDIT_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR => {
            sol_log("⚙️ Instruction: EditDataStorageAccount");

            let data_storage_pda_account_info = next_account_info(accounts_info)?;
            let authority_account_info = next_account_info(accounts_info)?;

            check_if_data_storage_account_is_immutable(data_storage_pda_account_info)?;

            check_account_is_signer(authority_account_info)?;

            // validate account's owner-program
            check_dsa_account_owner(
                data_storage_pda_account_info,
                program_id
            )?;

            // check that account is initialized
            check_dsa_account_is_initialized(data_storage_pda_account_info)?;

            // validate account's authority
            check_dsa_account_authority(
                data_storage_pda_account_info,
                authority_account_info.key.to_bytes()
            )?;

            // deserialize account data
            let dsa_data = data_storage_pda_account_info
                .data
                .try_borrow_mut()
                .unwrap();

            let label = dsa_data
                .get(32..62)
                .unwrap();
            let bump = *dsa_data
                .get(70)
                .unwrap();
            // validate PDA
            // Also we could validate authority_account & owner_program right here BUT to be developer friendly we seperated these checks!
            let seeds: &[&[u8]] = &[
                b"data_storage_account",
                authority_account_info.key.as_ref(),
                label,
                &[ bump ]
            ];
            create_and_check_program_address(
                seeds,
                program_id,
                data_storage_pda_account_info.key
            )?;

            drop(dsa_data);

            // update data-storage account
            // update 'last-updated' field
            let current_time = (Clock::get()?).unix_timestamp;
            sol_memcpy(
                data_storage_pda_account_info
                    .data
                    .try_borrow_mut()
                    .unwrap()
                    .get_mut(62..70)
                    .unwrap(),
                &current_time.to_le_bytes(),
                size_of::<i64>()
            );

            let old_data_length = u16::from_le_bytes(
                data_storage_pda_account_info
                    .data
                    .try_borrow_mut()
                    .unwrap()
                    .get(72..74)
                    .unwrap()
                    .try_into()
                    .unwrap()
            ) as usize;

            let new_data_length = ix_data.len();

            if new_data_length == old_data_length {
                // write new data
                sol_memcpy(
                    data_storage_pda_account_info
                        .data
                        .try_borrow_mut()
                        .unwrap()
                        .get_mut(74..)
                        .unwrap(),
                    ix_data,
                    old_data_length
                );
            } else if new_data_length < old_data_length {
                // write new data-length
                sol_memcpy(
                    data_storage_pda_account_info
                        .data
                        .try_borrow_mut()
                        .unwrap()
                        .get_mut(72..74)
                        .unwrap(),
                    &u16::to_le_bytes(new_data_length as u16),
                    size_of::<u16>()
                );

                // write new data
                sol_memcpy(
                    data_storage_pda_account_info
                        .data
                        .try_borrow_mut()
                        .unwrap()
                        .get_mut(74..)
                        .unwrap(),
                    ix_data,
                    new_data_length
                );

                // realloc account data
                calculate_new_dsa_size_and_realloc(
                    new_data_length,
                    old_data_length,
                    data_storage_pda_account_info,
                    new_data_length > old_data_length
                )?;

                // calculate rent_exempt lamports to refund
                let extra_rent_lamports = calculate_extra_rent_exempt_lamports(
                    old_data_length,
                    new_data_length,
                    new_data_length > old_data_length
                )?;

                // refund the extra rent_exempt
                let rent_receiver_account_info = next_account_info(accounts_info)?;

                **data_storage_pda_account_info.try_borrow_mut_lamports()? = data_storage_pda_account_info
                    .lamports()
                    .checked_sub(extra_rent_lamports)
                    .unwrap();

                **rent_receiver_account_info.try_borrow_mut_lamports()? = rent_receiver_account_info
                    .lamports()
                    .checked_add(extra_rent_lamports)
                    .unwrap();
            } else if new_data_length > old_data_length {
                // calculate rent_exempt lmaports to transfer to the data-account for extra-bytes
                let extra_rent_lamports = calculate_extra_rent_exempt_lamports(
                    old_data_length,
                    new_data_length,
                    new_data_length > old_data_length
                )?;

                // transfer lamports to the data-account
                let funding_account_info = next_account_info(accounts_info)?;
                let system_program_account_info = next_account_info(accounts_info)?;

                check_system_program_account(system_program_account_info.key)?;

                invoke(
                    &transfer_lamports(
                        funding_account_info.key,
                        data_storage_pda_account_info.key,
                        extra_rent_lamports
                    ),
                    &[
                        funding_account_info.clone(),
                        data_storage_pda_account_info.clone()
                    ]
                )?;

                // realloc extra bytes
                calculate_new_dsa_size_and_realloc(
                    new_data_length,
                    old_data_length,
                    data_storage_pda_account_info,
                    new_data_length > old_data_length
                )?;

                // write new data-length
                sol_memcpy(
                    data_storage_pda_account_info
                        .data
                        .try_borrow_mut()
                        .unwrap()
                        .get_mut(72..74)
                        .unwrap(),
                    &u16::to_le_bytes(new_data_length as u16),
                    size_of::<u16>()
                );

                // write new data
                sol_memcpy(
                    data_storage_pda_account_info
                        .data
                        .try_borrow_mut()
                        .unwrap()
                        .get_mut(74..)
                        .unwrap(),
                    ix_data,
                    new_data_length
                );
            };

            let event = Events::DataStorageAccountEdited {
                data_storage_account: *data_storage_pda_account_info.key,
                authority_account: *authority_account_info.key,
                old_data_len: old_data_length,
                new_data_len: new_data_length
            };
            emit!(event);

            sol_log("Data storage account has been updated successfully. ✅");
        },
        
        CLOSE_DATA_STORAGE_ACCOUNT_INSTRUCTION_DISCRIMINATOR => {
            sol_log("⚙️ Instruction: CloseDataStorageAccount");

            let data_storage_pda_account_info = next_account_info(accounts_info)?;
            let authority_account_info = next_account_info(accounts_info)?;
            let rent_receiver_account_info = next_account_info(accounts_info)?;

            check_if_data_storage_account_is_immutable(data_storage_pda_account_info)?;

            check_account_is_signer(authority_account_info)?;

            // validate account's owner-program
            check_dsa_account_owner(
                data_storage_pda_account_info,
                program_id
            )?;

            // check that account is initialized
            check_dsa_account_is_initialized(data_storage_pda_account_info)?;

            // validate account's authority
            check_dsa_account_authority(
                data_storage_pda_account_info,
                authority_account_info.key.to_bytes()
            )?;

            // deserialize account data
            let dsa_data = data_storage_pda_account_info
                .data
                .try_borrow_mut()
                .unwrap();

            let label = dsa_data
                .get(32..62)
                .unwrap();
            let bump = *dsa_data
                .get(70)
                .unwrap();

            // validate PDA
            // Also we could validate authority_account & owner_program right here BUT to be developer friendly we seperated these checks!
            create_and_check_program_address(
                &[
                    b"data_storage_account",
                    authority_account_info.key.as_ref(),
                    label,
                    &[ bump ]
                ],
                program_id,
                data_storage_pda_account_info.key
            )?;

            drop(dsa_data);

            // transfer dsa all lamports to the receiver-account
            let dsa_lamport_balance = data_storage_pda_account_info.lamports();

            **data_storage_pda_account_info.try_borrow_mut_lamports()? = 0;

            **rent_receiver_account_info.try_borrow_mut_lamports()? = rent_receiver_account_info
                .lamports()
                .checked_add(dsa_lamport_balance)
                .unwrap();

            // uninitialize the data-storage account
            let mut dsa_data = data_storage_pda_account_info
                .data
                .try_borrow_mut()
                .unwrap();
            let is_initialized_flag = dsa_data
                .get_mut(71)
                .unwrap();
            *is_initialized_flag = false as u8;

            let event = Events::DataStorageAccountClosed {
                data_storage_account: *data_storage_pda_account_info.key,
                authority_account: *authority_account_info.key
            };
            emit!(event);

            sol_log("Data storage account has been closed successfully. ✅");
        },
        _ => return Err(
            ProgramError::InvalidInstructionData
        )
    };

    Ok(())
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum DataStorageError {
    #[error("immutable data storage account.")]
    ImmutableDataStorage = 70,
    #[error("find_program_address failed!")]
    FailedToFindProgramAddress,
    #[error("invalid account-label (invalid utf-8)")]
    InvalidLabel,
    #[error("invalid data")]
    InvalidData
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Events {
    NewDataStorageAccountCreated {
        data_storage_account: Pubkey,
        authority_account: Pubkey,
        account_label: [u8; 30]
    },
    DataStorageAccountEdited {
        data_storage_account: Pubkey,
        authority_account: Pubkey,
        old_data_len: usize,
        new_data_len: usize
    },
    DataStorageAccountClosed {
        data_storage_account: Pubkey,
        authority_account: Pubkey
    }
}

mod helper {
    use super::{
        AccountInfo,
        Pubkey,
        invoke,
        invoke_signed,
        transfer_lamports,
        allocate_memory,
        assign_new_owner,
        ProgramError,
        ProgramResult,
        check_system_program_id,
        DataStorageError,
        SYSTEM_PROGRAM_ID,
        sol_memcmp,
        size_of
    };
    use solana_program::sysvar::{
        Sysvar,
        rent::Rent
    };

    pub(super) fn create_pda_account<'a, 'b>(
        new_pda_account_info: &AccountInfo<'a>,
        fee_payer_account_info: &AccountInfo<'b>,
        space: usize,
        program_id: &Pubkey,
        seeds: &[&[u8]]
    ) -> ProgramResult where 'b:'a, 'a:'b {
        let rent = Rent::get()?.minimum_balance(space);
        let new_pda_account_balance = new_pda_account_info.lamports();
        if new_pda_account_balance < rent {
            let lamports_needed = rent
                .checked_sub(new_pda_account_balance)
                .unwrap();
            
            invoke(
                &transfer_lamports(
                    fee_payer_account_info.key,
                    new_pda_account_info.key,
                    lamports_needed
                ),
                &[
                    fee_payer_account_info.clone(),
                    new_pda_account_info.clone()
                ]
            )?;
        };
    
        invoke_signed(
            &allocate_memory(
                new_pda_account_info.key,
                space as u64
            ),
            &[ new_pda_account_info.clone() ],
            &[ seeds ]
        )?;
    
        invoke_signed(
            &assign_new_owner(
                new_pda_account_info.key,
                program_id
            ),
            &[ new_pda_account_info.clone() ],
            &[ seeds ]
        )?;
    
        Ok(())
    }
    
    pub(super) fn check_account_is_signer(account_info: &AccountInfo) -> ProgramResult {
        if account_info.is_signer == false {
            return Err(
                ProgramError::MissingRequiredSignature
            );
        };
    
        Ok(())
    }
    
    pub(super) fn check_system_program_account(expected_program_id: &Pubkey) -> ProgramResult {
        if check_system_program_id(expected_program_id) == false {
            return Err(
                ProgramError::IncorrectProgramId
            );
        };
    
        Ok(())
    }
    
    // NOTE: If a data-storage account's authority is SYSTEM_PROGRAM_ACCOUNT thix means that the dsa is an immutable-account and it's authority cannot be a signer BUT
    //  to be developer friendly we add this check to make the code more beautiful !
    pub(super) fn check_if_data_storage_account_is_immutable(data_storage_account_info: &AccountInfo) -> ProgramResult {
        let cmp_result = sol_memcmp(
            data_storage_account_info
                .data
                .try_borrow()
                .unwrap()
                .get(..32)
                .unwrap(),
            &SYSTEM_PROGRAM_ID.to_bytes(),
            size_of::<Pubkey>()
        );
    
        if cmp_result == 0 {
            return Err(
                ProgramError::Custom(
                    DataStorageError::ImmutableDataStorage as u32
                )
            )
        };
    
        Ok(())
    }
    
    pub(super) fn check_dsa_account_owner(
        data_storage_account_info: &AccountInfo,
        expected_owner: &Pubkey
    ) -> ProgramResult {
        if data_storage_account_info.owner != expected_owner {
            return Err(
                ProgramError::InvalidAccountOwner
            );
        };
    
        Ok(())
    }
    
    pub(super) fn create_and_check_program_address(
        seeds: &[&[u8]],
        program_id: &Pubkey,
        expected_data_storage_pda_account_pubkey: &Pubkey
    ) -> ProgramResult {
        let dsa_pda_addr = Pubkey::create_program_address(
            seeds,
            program_id
        ).map_err::<ProgramError, _>(|_| ProgramError::Custom(DataStorageError::FailedToFindProgramAddress as u32))?;
    
        if &dsa_pda_addr != expected_data_storage_pda_account_pubkey {
            return Err(
                ProgramError::InvalidAccountData
            );
        };
    
        Ok(())
    }
    
    pub(super) fn check_dsa_account_authority(
        data_storage_account_info: &AccountInfo,
        expected_authority_pubkey: [u8; 32]
    ) -> ProgramResult {
        let cmp_result = sol_memcmp(
            data_storage_account_info
                .data
                .try_borrow()
                .unwrap()
                .get(..32)
                .unwrap(),
            expected_authority_pubkey.as_slice(),
            size_of::<Pubkey>()
        );
    
        if cmp_result != 0 {
            return Err(
                ProgramError::IncorrectAuthority
            );
        };
    
        Ok(())
    }
    
    pub(super) fn check_dsa_account_is_initialized(data_storage_account_info: &AccountInfo) -> ProgramResult {
        let dsa_data = data_storage_account_info
            .data
            .try_borrow()
            .unwrap();
    
        let is_initialized_flag = *dsa_data
            .get(71)
            .unwrap();
    
        if is_initialized_flag == false as u8 {
            return Err(
                ProgramError::UninitializedAccount
            );
        };
    
        Ok(())
    }
    
    pub(super) fn calculate_extra_rent_exempt_lamports(
        old_data_length: usize,
        new_data_length: usize,
        new_is_bigger: bool
    ) -> Result<u64, ProgramError> {
        let extra_bytes_len: usize;
        if new_is_bigger == false {
            extra_bytes_len = old_data_length
            .checked_sub(new_data_length)
            .unwrap();
        } else {
            extra_bytes_len = new_data_length
            .checked_sub(old_data_length)
            .unwrap();
        };
    
        let rent_sysvar = Rent::get()?;
    
        let extra_rent_lamports = (
            rent_sysvar
                .lamports_per_byte_year
                .checked_mul(extra_bytes_len as u64)
                .unwrap()
        ).checked_mul(rent_sysvar.exemption_threshold as u64).unwrap();
    
        Ok(extra_rent_lamports)
    }
    
    pub(super) fn calculate_new_dsa_size_and_realloc(
        new_data_len: usize,
        old_data_len: usize,
        data_storage_pda_account_info: &AccountInfo,
        new_is_bigger: bool
    ) -> ProgramResult {
        let new_dsa_size: usize;
        if new_is_bigger == false {
            let extra_bytes = old_data_len
                .checked_sub(new_data_len)
                .unwrap();
    
            new_dsa_size = data_storage_pda_account_info
                .data_len()
                .checked_sub(extra_bytes)
                .unwrap();
        } else {
            let extra_bytes = new_data_len
                .checked_sub(old_data_len)
                .unwrap();
        
            new_dsa_size = data_storage_pda_account_info
                .data_len()
                .checked_add(extra_bytes)
                .unwrap();
        };
    
        data_storage_pda_account_info.realloc(
            new_dsa_size,
            false
        )?;
    
        Ok(())
    }
}