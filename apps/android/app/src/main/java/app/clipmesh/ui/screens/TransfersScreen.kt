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

data class TransferRecord(
    val id: String,
    val fileName: String,
    val sizeText: String,
    val progress: Float,
    val status: String,
    val peerName: String,
    val isOutbound: Boolean,
    val sha256Verified: Boolean
)

@Composable
fun TransfersScreen(
    onSendFile: () -> Unit
) {
    var transfers by remember {
        mutableStateOf(
            listOf(
                TransferRecord("1", "presentation_v2.pdf", "14.2 MB", 1.0f, "Completed", "Windows Desktop", false, true),
                TransferRecord("2", "camera_raw_backup.zip", "184.0 MB", 0.65f, "Transferring", "ThinkPad X1", true, false),
                TransferRecord("3", "app-release.apk", "32.8 MB", 1.0f, "Completed", "Linux Workstation", false, true)
            )
        )
    }

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
                text = "Secure File Transfers",
                color = Slate100,
                fontSize = 16.sp,
                fontWeight = FontWeight.SemiBold
            )
            Button(
                onClick = onSendFile,
                colors = ButtonDefaults.buttonColors(containerColor = Indigo600),
                shape = RoundedCornerShape(8.dp),
                contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp)
            ) {
                Text("+ Send File", fontSize = 12.sp)
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        LazyColumn(
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            items(transfers, key = { it.id }) { transfer ->
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
                            Column(modifier = Modifier.weight(1f)) {
                                Text(
                                    text = transfer.fileName,
                                    color = Slate100,
                                    fontSize = 14.sp,
                                    fontWeight = FontWeight.Medium
                                )
                                Spacer(modifier = Modifier.height(2.dp))
                                Text(
                                    text = "${if (transfer.isOutbound) "To: " else "From: "}${transfer.peerName} • ${transfer.sizeText}",
                                    color = Slate400,
                                    fontSize = 11.sp
                                )
                            }

                            Box(
                                modifier = Modifier
                                    .clip(RoundedCornerShape(6.dp))
                                    .background(
                                        if (transfer.status == "Completed") Emerald500.copy(alpha = 0.2f)
                                        else Indigo500.copy(alpha = 0.2f)
                                    )
                                    .padding(horizontal = 8.dp, vertical = 3.dp)
                            ) {
                                Text(
                                    text = transfer.status.uppercase(),
                                    color = if (transfer.status == "Completed") Emerald500 else Indigo500,
                                    fontSize = 10.sp,
                                    fontWeight = FontWeight.Bold
                                )
                            }
                        }

                        Spacer(modifier = Modifier.height(10.dp))

                        // Progress Bar
                        LinearProgressIndicator(
                            progress = { transfer.progress },
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(6.dp)
                                .clip(RoundedCornerShape(3.dp)),
                            color = if (transfer.status == "Completed") Emerald500 else Indigo500,
                            trackColor = Slate800,
                        )

                        Spacer(modifier = Modifier.height(8.dp))

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            if (transfer.sha256Verified) {
                                Text(
                                    text = "✓ SHA-256 Verified",
                                    color = Emerald500,
                                    fontSize = 11.sp,
                                    fontWeight = FontWeight.Medium
                                )
                            } else {
                                Text(
                                    text = "${(transfer.progress * 100).toInt()}% • 64KB chunks AEAD",
                                    color = Slate400,
                                    fontSize = 11.sp
                                )
                            }

                            if (transfer.status == "Completed") {
                                TextButton(
                                    onClick = { /* Open file */ },
                                    contentPadding = PaddingValues(horizontal = 8.dp, vertical = 2.dp)
                                ) {
                                    Text("Open", color = Indigo500, fontSize = 12.sp)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

