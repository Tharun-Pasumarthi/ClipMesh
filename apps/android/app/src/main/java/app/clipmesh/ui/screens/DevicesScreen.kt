package app.clipmesh.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
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

data class DeviceItem(
    val id: String,
    val name: String,
    val platform: String,
    val fingerprint: String,
    val isOnline: Boolean,
    val isCurrent: Boolean,
    val isRevoked: Boolean
)

@Composable
fun DevicesScreen(
    onNavigateToPairing: () -> Unit
) {
    var devices by remember {
        mutableStateOf(
            listOf(
                DeviceItem("1", "Pixel 8 Pro", "Android", "e4f8a1...2b9c", true, isCurrent = true, isRevoked = false),
                DeviceItem("2", "ThinkPad X1 Carbon", "Windows", "a9b8c7...3f2e", true, isCurrent = false, isRevoked = false),
                DeviceItem("3", "MacBook Pro M3", "macOS", "c1d2e3...8a7b", false, isCurrent = false, isRevoked = false)
            )
        )
    }

    var deviceToRevoke by remember { mutableStateOf<DeviceItem?>(null) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp)
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "Trusted Devices (${devices.filter { !it.isRevoked }.size})",
                color = Slate100,
                fontSize = 16.sp,
                fontWeight = FontWeight.SemiBold
            )
            Button(
                onClick = onNavigateToPairing,
                colors = ButtonDefaults.buttonColors(containerColor = Indigo600),
                shape = RoundedCornerShape(8.dp),
                contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp)
            ) {
                Text("+ Pair New", fontSize = 12.sp)
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        LazyColumn(
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            items(devices, key = { it.id }) { dev ->
                Card(
                    colors = CardDefaults.cardColors(containerColor = Slate900),
                    shape = RoundedCornerShape(10.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(modifier = Modifier.padding(14.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Column {
                                Row(
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                                ) {
                                    Text(
                                        text = dev.name,
                                        color = Slate100,
                                        fontSize = 14.sp,
                                        fontWeight = FontWeight.SemiBold
                                    )
                                    if (dev.isCurrent) {
                                        Box(
                                            modifier = Modifier
                                                .clip(RoundedCornerShape(4.dp))
                                                .background(Indigo500.copy(alpha = 0.2f))
                                                .padding(horizontal = 6.dp, vertical = 2.dp)
                                        ) {
                                            Text("THIS DEVICE", color = Indigo500, fontSize = 9.sp, fontWeight = FontWeight.Bold)
                                        }
                                    }
                                }
                                Spacer(modifier = Modifier.height(2.dp))
                                Text(
                                    text = dev.platform,
                                    color = Slate400,
                                    fontSize = 11.sp
                                )
                            }

                            Box(
                                modifier = Modifier
                                    .clip(RoundedCornerShape(6.dp))
                                    .background(
                                        if (dev.isRevoked) Rose500.copy(alpha = 0.2f)
                                        else if (dev.isOnline) Emerald500.copy(alpha = 0.2f)
                                        else Slate800
                                    )
                                    .padding(horizontal = 8.dp, vertical = 3.dp)
                            ) {
                                Text(
                                    text = if (dev.isRevoked) "REVOKED" else if (dev.isOnline) "ONLINE" else "OFFLINE",
                                    color = if (dev.isRevoked) Rose500 else if (dev.isOnline) Emerald500 else Slate400,
                                    fontSize = 10.sp,
                                    fontWeight = FontWeight.Bold
                                )
                            }
                        }

                        Spacer(modifier = Modifier.height(10.dp))
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Column {
                                Text("Public Key Fingerprint", color = Slate400, fontSize = 10.sp)
                                Text(
                                    text = dev.fingerprint,
                                    color = Slate200,
                                    fontSize = 11.sp,
                                    fontFamily = FontFamily.Monospace
                                )
                            }

                            if (!dev.isCurrent && !dev.isRevoked) {
                                OutlinedButton(
                                    onClick = { deviceToRevoke = dev },
                                    colors = ButtonDefaults.outlinedButtonColors(contentColor = Rose500),
                                    shape = RoundedCornerShape(6.dp),
                                    contentPadding = PaddingValues(horizontal = 10.dp, vertical = 2.dp)
                                ) {
                                    Text("Revoke", fontSize = 11.sp)
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Revocation Confirmation Dialog
    deviceToRevoke?.let { target ->
        AlertDialog(
            onDismissRequest = { deviceToRevoke = null },
            title = { Text("Revoke Device Access?", color = Slate100, fontSize = 16.sp) },
            text = {
                Text(
                    "Are you sure you want to revoke '${target.name}'? It will immediately lose access to encrypted clipboard synchronisation and file transfers.",
                    color = Slate300,
                    fontSize = 13.sp
                )
            },
            confirmButton = {
                Button(
                    onClick = {
                        devices = devices.map {
                            if (it.id == target.id) it.copy(isRevoked = true, isOnline = false) else it
                        }
                        deviceToRevoke = null
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = Rose500),
                    shape = RoundedCornerShape(6.dp)
                ) {
                    Text("Revoke Access")
                }
            },
            dismissButton = {
                TextButton(onClick = { deviceToRevoke = null }) {
                    Text("Cancel", color = Slate400)
                }
            },
            containerColor = Slate900,
            shape = RoundedCornerShape(12.dp)
        )
    }
}

