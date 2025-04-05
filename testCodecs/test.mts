import {
    createSolanaRpc,
    createKeyPairSignerFromBytes,
    pipe,
    createTransactionMessage,
    setTransactionMessageLifetimeUsingBlockhash,
    setTransactionMessageFeePayerSigner,
    appendTransactionMessageInstruction,
    getProgramDerivedAddress,
    getAddressEncoder,
    getUtf8Encoder,
    Address,
    signTransactionMessageWithSigners,
    getSignatureFromTransaction,
    createSolanaRpcSubscriptions,
    sendAndConfirmTransactionFactory,
    getBase64Encoder
} from "@solana/kit";

import { expect } from "chai";

import { getDataStorageAccountDecoder } from "./getCodecs.mjs";
import {
    getCreateDataStorageAccountInstruction,
    getEditDataStorageAccountWithNewDataLenIsEqualToOldDataLenInstruction,
    getCloseDataStorageAccountInstruction
} from "./getIxs.mjs";


(async () => {
    // localhost
    const RPC = createSolanaRpc(
        "http://127.0.0.1:8899/"
    );
    const RPC_SUBSCRIPTIONS = createSolanaRpcSubscriptions(
        "ws://127.0.0.1:8900/"
    );

    const USER_KEYPAIR = await createKeyPairSignerFromBytes(
        Uint8Array.from(
            [ "<KEYPAIR-BYTES>" ]
        )
    );
    const DATA_STORAGE_PROGRAM_ID = "DSAgnFyNE53P9m5vz9ALojPQwbtaPjwzPa61ZN1oe7mG" as Address;

    // Data Storage PDA
    const label = "+PooriaGG + Solana + + + + + )";
    const data = new Uint8Array([ 12, 12, 56, 42, 12, 99, 88, 77, 56, 75 ]);
    const [ dsa_pda_addr ] = await getProgramDerivedAddress(
        {
            seeds: [
                getUtf8Encoder().encode("data_storage_account"),
                getAddressEncoder().encode(USER_KEYPAIR.address),
                getUtf8Encoder().encode(label)
            ],
            programAddress: DATA_STORAGE_PROGRAM_ID
        }
    );

    /// Create New Data-Storage PDA 
    {
        const latestBlockhash = (await RPC.getLatestBlockhash({ commitment: "confirmed" }).send()).value;
        const createAccountIx = getCreateDataStorageAccountInstruction(
            {
                data,
                data_storage_authority: USER_KEYPAIR.address,
                funding_account: USER_KEYPAIR.address,
                label,
                new_data_storage_pda: dsa_pda_addr,
            }
        );
        const transactionMessage = pipe(
            createTransactionMessage({ version: "legacy" }),
            txMsg => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, txMsg),
            txMsg => setTransactionMessageFeePayerSigner(USER_KEYPAIR, txMsg),
            txMsg => appendTransactionMessageInstruction(
                createAccountIx,
                txMsg
            )
        );

        const fullySignedTransaction = await signTransactionMessageWithSigners(transactionMessage);

        const transactionSignature = getSignatureFromTransaction(fullySignedTransaction);
        console.log(`\nTransaction Signature - ${transactionSignature}`);

        await sendAndConfirmTransactionFactory(
            {
                rpc: RPC,
                rpcSubscriptions: RPC_SUBSCRIPTIONS
            }
        )(fullySignedTransaction, { commitment: "confirmed" });

        const [
            txStatus,
            accountInfo
        ] = await Promise.all(
            [
                RPC.getSignatureStatuses([ transactionSignature ]).send(),
                RPC.getAccountInfo(dsa_pda_addr, { commitment: "confirmed", encoding: "base64" }).send()
            ]
        );

        console.log("\nTransaction Status -", txStatus?.value[0]?.confirmationStatus);

        const parsedAccountData = getDataStorageAccountDecoder().decode(
            // @ts-ignore
            getBase64Encoder().encode(accountInfo?.value.data[0])
        );
        console.log("\nNew Data Storage Account :\n", parsedAccountData);

        expect(parsedAccountData.authority).to.be.eq(USER_KEYPAIR.address);
        expect(JSON.stringify(parsedAccountData.data)).to.be.eq(JSON.stringify(Array.from(data) as number[]));
        expect(parsedAccountData.isInitialized).to.be.eq(true);
        expect(parsedAccountData.label).to.be.eq(label);
        expect(parsedAccountData.lastUpdated).to.be.eq(0n);
    }
    /// Create New Data-Storage PDA

    /// Edit Created Data-Storage PDA
    {
        const latestBlockhash = (await RPC.getLatestBlockhash({ commitment: "confirmed" }).send()).value;
        const newData = new Uint8Array([ 78, 12, 56, 42, 1, 120, 2, 77, 88, 99 ]);
        const editCreatedAccountIx = getEditDataStorageAccountWithNewDataLenIsEqualToOldDataLenInstruction(
            {
                data_storage_authority: USER_KEYPAIR.address,
                data_storage_pda: dsa_pda_addr,
                new_data: newData
            }
        );
        const transactionMessage = pipe(
            createTransactionMessage({ version: "legacy" }),
            txMsg => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, txMsg),
            txMsg => setTransactionMessageFeePayerSigner(USER_KEYPAIR, txMsg),
            txMsg => appendTransactionMessageInstruction(
                editCreatedAccountIx,
                txMsg
            )
        );

        const fullySignedTransaction = await signTransactionMessageWithSigners(transactionMessage);

        const transactionSignature = getSignatureFromTransaction(fullySignedTransaction);
        console.log(`\nTransaction Signature - ${transactionSignature}`);

        await sendAndConfirmTransactionFactory(
            {
                rpc: RPC,
                rpcSubscriptions: RPC_SUBSCRIPTIONS
            }
        )(fullySignedTransaction, { commitment: "confirmed" });

        const [
            txStatus,
            accountInfo
        ] = await Promise.all(
            [
                RPC.getSignatureStatuses([ transactionSignature ]).send(),
                RPC.getAccountInfo(dsa_pda_addr, { commitment: "confirmed", encoding: "base64" }).send()
            ]
        );

        console.log("\nTransaction Status -", txStatus?.value[0]?.confirmationStatus);

        const parsedAccountData = getDataStorageAccountDecoder().decode(
            // @ts-ignore
            getBase64Encoder().encode(accountInfo?.value.data[0])
        );
        console.log("\nNew Data Storage Account :\n", parsedAccountData);

        expect(JSON.stringify(parsedAccountData.data)).to.be.eq(JSON.stringify(Array.from(newData) as number[]));
        expect(parsedAccountData.lastUpdated).not.to.eq(0n);
    }
    /// Edit Created Data-Storage PDA

    /// Close Editted Data-Storage PDA
    {
        const latestBlockhash = (await RPC.getLatestBlockhash({ commitment: "confirmed" }).send()).value;
        const closeEdittedAccountIx = getCloseDataStorageAccountInstruction(
            {
                data_storage_authority: USER_KEYPAIR.address,
                data_storage_pda: dsa_pda_addr,
                rent_exempt_receiver_account: USER_KEYPAIR.address
            }
        );
        const transactionMessage = pipe(
            createTransactionMessage({ version: "legacy" }),
            txMsg => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, txMsg),
            txMsg => setTransactionMessageFeePayerSigner(USER_KEYPAIR, txMsg),
            txMsg => appendTransactionMessageInstruction(
                closeEdittedAccountIx,
                txMsg
            )
        );

        const fullySignedTransaction = await signTransactionMessageWithSigners(transactionMessage);

        const transactionSignature = getSignatureFromTransaction(fullySignedTransaction);
        console.log(`\nTransaction Signature - ${transactionSignature}`);

        await sendAndConfirmTransactionFactory(
            {
                rpc: RPC,
                rpcSubscriptions: RPC_SUBSCRIPTIONS
            }
        )(fullySignedTransaction, { commitment: "confirmed" });

        const [
            txStatus,
            { value }
        ] = await Promise.all(
            [
                RPC.getSignatureStatuses([ transactionSignature ]).send(),
                RPC.getAccountInfo(dsa_pda_addr, { commitment: "confirmed", encoding: "base64" }).send()
            ]
        );

        console.log("\nTransaction Status -", txStatus?.value[0]?.confirmationStatus);
        
        expect(value).to.be.eq(null, "Account must be deleted so far!");
    }
    /// Close Editted Data-Storage PDA
})();