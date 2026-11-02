package app.clipmesh.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.clipmesh.ui.theme.*

@Composable
fun PairingScreen(
    onPairingSuccess: () -> Unit
) {
    var manualCode by remember { mutableStateOf("") }
    var showApprovalDialog by remember { mutableStateOf(false) }
    var secondsLeft by remember { mutableStateOf(120) }

    LaunchedEffect(Unit) {
        while (secondsLeft > 0) {
            kotlinx.coroutines.delay(1000)
            secondsLeft -= 1
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Text(
            text = "Pair New Device",
            color = Slate100,
            fontSize = 18.sp,
            fontWeight = FontWeight.SemiBold
        )
        Spacer(modifier = Modifier.height(4.dp))
        Text(
            text = "Scan the QR code displayed on your ClipMesh Desktop screen",
            color = Slate400,
            fontSize = 12.sp,
            textAlign = TextAlign.Center
        )

        Spacer(modifier = Modifier.height(20.dp))

        // Camera Viewfinder Mock / Viewfinder Area
        Box(
            modifier = Modifier
                .size(240.dp)
                .clip(RoundedCornerShape(16.dp))
                .background(Slate900)
                .border(2.dp, Indigo500, RoundedCornerShape(16.dp)),
            contentAlignment = Alignment.Center
        ) {
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center
            ) {
                Text(
                    text = "[ Camera Viewfinder ]",
                    color = Slate400,
                    fontSize = 13.sp
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = "Point camera at desktop QR",
                    color = Slate400,
                    fontSize = 11.sp
                )
                Spacer(modifier = Modifier.height(16.dp))
                Button(
                    onClick = { showApprovalDialog = true },
                    colors = ButtonDefaults.buttonColors(containerColor = Indigo600),
                    shape = RoundedCornerShape(8.dp),
                    contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp)
                ) {
                    Text("Simulate QR Scan", fontSize = 11.sp)
                }
            }
        }

        Spacer(modifier = Modifier.height(12.dp))

        // Expiry Timer Indicator
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp)
        ) {
            Text(
                text = "Session expires in: ${secondsLeft}s",
                color = if (secondsLeft < 30) Rose500 else Amber500,
                fontSize = 12.sp,
                fontWeight = FontWeight.Medium
            )
        }

        Spacer(modifier = Modifier.height(24.dp))

        // Manual 6-Digit Code Fallback
        Text(
            text = "Or enter 6-digit confirmation code",
            color = Slate400,
            fontSize = 12.sp
        )
        Spacer(modifier = Modifier.height(8.dp))

        OutlinedTextField(
            value = manualCode,
            onValueChange = { if (it.length <= 6) manualCode = it },
            placeholder = { Text("000000", color = Slate400, fontSize = 14.sp) },
            colors = OutlinedTextFieldDefaults.colors(
                focusedContainerColor = Slate900,
                unfocusedContainerColor = Slate900,
                focusedBorderColor = Indigo500,
                unfocusedBorderColor = Slate800,
                focusedTextColor = Slate100,
                unfocusedTextColor = Slate100
            ),
            shape = RoundedCornerShape(8.dp),
            modifier = Modifier.width(180.dp),
            textStyle = androidx.compose.ui.text.TextStyle(
                fontFamily = FontFamily.Monospace,
                fontSize = 16.sp,
                textAlign = TextAlign.Center,
                letterSpacing = 4.sp
            )
        )

        Spacer(modifier = Modifier.height(12.dp))

        Button(
            onClick = {
                if (manualCode.length == 6) {
                    showApprovalDialog = true
                }
            },
            enabled = manualCode.length == 6,
            colors = ButtonDefaults.buttonColors(containerColor = Indigo600),
            shape = RoundedCornerShape(8.dp),
            modifier = Modifier.width(180.dp)
        ) {
            Text("Verify Code")
        }
    }

    // Mutual Approval Dialog
    if (showApprovalDialog) {
        AlertDialog(
            onDismissRequest = { showApprovalDialog = false },
            title = {
                Text(
                    text = "Approve Pairing Request",
                    color = Slate100,
                    fontSize = 16.sp,
                    fontWeight = FontWeight.SemiBold
                )
            },
            text = {
                Column {
                    Text(
                        text = "Device 'ThinkPad X1 Carbon' is requesting to join your ClipMesh cluster.",
                        color = Slate300,
                        fontSize = 13.sp
                    )
                    Spacer(modifier = Modifier.height(12.dp))
                    Text(
                        text = "Verification Fingerprint:",
                        color = Slate400,
                        fontSize = 11.sp
                    )
                    Text(
                        text = "b4a7-89ef-33cd-1209",
                        color = Indigo500,
                        fontSize = 13.sp,
                        fontFamily = FontFamily.Monospace,
                        fontWeight = FontWeight.Bold
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = "Confirm that this fingerprint matches the code shown on your laptop screen.",
                        color = Slate400,
                        fontSize = 11.sp
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        showApprovalDialog = false
                        onPairingSuccess()
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = Emerald500),
                    shape = RoundedCornerShape(6.dp)
                ) {
                    Text("Approve & Trust")
                }
            },
            dismissButton = {
                TextButton(onClick = { showApprovalDialog = false }) {
                    Text("Reject", color = Rose500)
                }
            },
            containerColor = Slate900,
            shape = RoundedCornerShape(12.dp)
        )
    }
}

