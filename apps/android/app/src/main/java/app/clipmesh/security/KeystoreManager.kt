package app.clipmesh.security

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.security.KeyPair
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.PrivateKey
import java.security.PublicKey
import java.security.Signature

/**
 * Android Keystore hardware-backed key manager.
 * Securely generates and stores device identity keys in the device's TEE or StrongBox.
 */
class KeystoreManager {
    companion object {
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val KEY_ALIAS_IDENTITY = "clipmesh_identity_ed25519"
        private const val KEY_ALIAS_AGREEMENT = "clipmesh_agreement_ec"
    }

    private val keyStore: KeyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply {
        load(null)
    }

    /**
     * Generate hardware-isolated keypair in Android TEE
     */
    fun getOrCreateIdentityKeyPair(): KeyPair {
        if (!keyStore.containsAlias(KEY_ALIAS_IDENTITY)) {
            val kpg = KeyPairGenerator.getInstance(
                KeyProperties.KEY_ALGORITHM_EC,
                ANDROID_KEYSTORE
            )
            val parameterSpec = KeyGenParameterSpec.Builder(
                KEY_ALIAS_IDENTITY,
                KeyProperties.PURPOSE_SIGN or KeyProperties.PURPOSE_VERIFY
            )
                .setDigests(KeyProperties.DIGEST_SHA256)
                .setUserAuthenticationRequired(false) // Can be set to true for biometric approval
                .build()

            kpg.initialize(parameterSpec)
            return kpg.generateKeyPair()
        }

        val privateKey = keyStore.getKey(KEY_ALIAS_IDENTITY, null) as PrivateKey
        val publicKey = keyStore.getCertificate(KEY_ALIAS_IDENTITY).publicKey
        return KeyPair(publicKey, privateKey)
    }

    /**
     * Sign proof-of-possession challenge with device hardware private key
     */
    fun signChallenge(challenge: ByteArray): ByteArray {
        val privateKey = (keyStore.getKey(KEY_ALIAS_IDENTITY, null) as? PrivateKey)
            ?: getOrCreateIdentityKeyPair().private

        val signature = Signature.getInstance("SHA256withECDSA")
        signature.initSign(privateKey)
        signature.update(challenge)
        return signature.sign()
    }
}

