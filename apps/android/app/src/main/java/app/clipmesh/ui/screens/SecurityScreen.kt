package app.clipmesh.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.clipmesh.ui.theme.*

@Composable
fun SecurityScreen(
    onClearHistory: () -> Unit,
    onResetIdentity: () -> Unit
) {
    var foregroundServiceEnabled by remember { mutableStateOf(true) }
    var biometricUnlockEnabled by remember { mutableStateOf(false) }
    var autoClearUnpinned by remember { mutableStateOf(true) }
    var retentionDays by remember { mutableStateOf(30) }

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        item {
            Text(
                text = "Security & Privacy",
                color = Slate100,
                fontSize = 18.sp,
                fontWeight = FontWeight.SemiBold
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = "Hardware-backed keys and zero-knowledge synchronization settings",
                color = Slate400,
                fontSize = 12.sp
            )
        }

        // Cryptographic Status Card
        item {
            Card(
                colors = CardDefaults.cardColors(containerColor = Slate900),
                shape = RoundedCornerShape(10.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    Text(
                        text = "Cryptographic Architecture",
                        color = Slate100,
                        fontSize = 14.sp,
                        fontWeight = FontWeight.SemiBold
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    CryptoDetailRow("Cipher Suite", "ChaCha20-Poly1305 AEAD")
                    CryptoDetailRow("Key Agreement", "X25519 ECDH + HKDF-SHA256")
                    CryptoDetailRow("Signatures", "Ed25519 (256-bit)")
                    CryptoDetailRow("Secure Storage", "Android Keystore (StrongBox)")
                    CryptoDetailRow("Relay Architecture", "Zero-Knowledge Blind Envelope")
                }
            }
        }

        // Permissions & Services
        item {
            Card(
                colors = CardDefaults.cardColors(containerColor = Slate900),
                shape = RoundedCornerShape(10.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    Text(
                        text = "Clipboard Background Service",
                        color = Slate100,
                        fontSize = 14.sp,
                        fontWeight = FontWeight.SemiBold
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                    Text(
                        text = "Android 10+ requires a persistent foreground service with user notification to monitor clipboard changes.",
                        color = Slate400,
                        fontSize = 11.sp
                    )
                    Spacer(modifier = Modifier.height(10.dp))
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text("Active Foreground Service", color = Slate200, fontSize = 13.sp)
                        Switch(
                            checked = foregroundServiceEnabled,
                            onCheckedChange = { foregroundServiceEnabled = it },
                            colors = SwitchDefaults.colors(
                                checkedThumbColor = Slate100,
                                checkedTrackColor = Indigo600,
                                uncheckedThumbColor = Slate400,
                                uncheckedTrackColor = Slate800
                            )
                        )
                    }
                }
            }
        }

        // Data Retention Settings
        item {
            Card(
                colors = CardDefaults.cardColors(containerColor = Slate900),
                shape = RoundedCornerShape(10.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    Text(
                        text = "History Retention",
                        color = Slate100,
                        fontSize = 14.sp,
                        fontWeight = FontWeight.SemiBold
                    )
                    Spacer(modifier = Modifier.height(10.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text("Auto-clear unpinned items", color = Slate200, fontSize = 13.sp)
                        Switch(
                            checked = autoClearUnpinned,
                            onCheckedChange = { autoClearUnpinned = it },
                            colors = SwitchDefaults.colors(
                                checkedThumbColor = Slate100,
                                checkedTrackColor = Indigo600,
                                uncheckedThumbColor = Slate400,
                                uncheckedTrackColor = Slate800
                            )
                        )
                    }

                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = "Retention period: $retentionDays days",
                        color = Slate400,
                        fontSize = 12.sp
                    )
                    Slider(
                        value = retentionDays.toFloat(),
                        onValueChange = { retentionDays = it.toInt() },
                        valueRange = 7f..90f,
                        steps = 2,
                        colors = SliderDefaults.colors(
                            thumbColor = Indigo500,
                            activeTrackColor = Indigo600,
                            inactiveTrackColor = Slate800
                        )
                    )
                }
            }
        }

        // Danger Zone
        item {
            Card(
                colors = CardDefaults.cardColors(containerColor = Slate900),
                shape = RoundedCornerShape(10.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    Text(
                        text = "Danger Zone",
                        color = Rose500,
                        fontSize = 14.sp,
                        fontWeight = FontWeight.SemiBold
                    )
                    Spacer(modifier = Modifier.height(10.dp))

                    OutlinedButton(
                        onClick = onClearHistory,
                        colors = ButtonDefaults.outlinedButtonColors(contentColor = Amber500),
                        shape = RoundedCornerShape(8.dp),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("Clear All Local History")
                    }

                    Spacer(modifier = Modifier.height(8.dp))

                    Button(
                        onClick = onResetIdentity,
                        colors = ButtonDefaults.buttonColors(containerColor = Rose500),
                        shape = RoundedCornerShape(8.dp),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("Reset Cryptographic Device Identity")
                    }
                }
            }
        }
    }
}

@Composable
private fun CryptoDetailRow(label: String, value: String) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        horizontalArrangement = Arrangement.SpaceBetween
    ) {
        Text(text = label, color = Slate400, fontSize = 12.sp)
        Text(text = value, color = Slate200, fontSize = 12.sp, fontWeight = FontWeight.Medium)
    }
}

