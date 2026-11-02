package app.clipmesh.ui

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.clipmesh.ui.screens.*
import app.clipmesh.ui.theme.*

enum class AppTab(val label: String) {
    OVERVIEW("Overview"),
    HISTORY("History"),
    DEVICES("Devices"),
    PAIRING("Pair"),
    TRANSFERS("Transfers"),
    SECURITY("Security")
}

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            ClipMeshTheme {
                MainAppScaffold(
                    onCopyToClipboard = { text ->
                        val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                        val clip = ClipData.newPlainText("ClipMesh", text)
                        clipboard.setPrimaryClip(clip)
                        Toast.makeText(this, "Copied to clipboard", Toast.LENGTH_SHORT).show()
                    }
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MainAppScaffold(
    onCopyToClipboard: (String) -> Unit
) {
    var selectedTab by remember { mutableStateOf(AppTab.OVERVIEW) }
    val snackbarHostState = remember { SnackbarHostState() }
    val coroutineScope = rememberCoroutineScope()

    Scaffold(
        snackbarHost = { SnackbarHost(snackbarHostState) },
        topBar = {
            TopAppBar(
                title = {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Text(
                            text = "ClipMesh",
                            color = Slate100,
                            fontSize = 18.sp,
                            fontWeight = androidx.compose.ui.text.font.FontWeight.Bold
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = Slate950,
                    titleContentColor = Slate100
                )
            )
        },
        bottomBar = {
            NavigationBar(
                containerColor = Slate950,
                tonalElevation = 8.dp
            ) {
                AppTab.values().forEach { tab ->
                    NavigationBarItem(
                        selected = selectedTab == tab,
                        onClick = { selectedTab = tab },
                        label = { Text(tab.label, fontSize = 10.sp) },
                        icon = {
                            // Simple indicator dot
                            Box(
                                modifier = Modifier
                                    .size(6.dp)
                                    .background(
                                        if (selectedTab == tab) Indigo500 else Slate700,
                                        shape = androidx.compose.foundation.shape.CircleShape
                                    )
                            )
                        },
                        colors = NavigationBarItemDefaults.colors(
                            selectedTextColor = Indigo500,
                            unselectedTextColor = Slate400,
                            indicatorColor = Slate900
                        )
                    )
                }
            }
        },
        containerColor = Slate950
    ) { innerPadding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
        ) {
            when (selectedTab) {
                AppTab.OVERVIEW -> OverviewScreen(
                    onNavigateToHistory = { selectedTab = AppTab.HISTORY },
                    onNavigateToPairing = { selectedTab = AppTab.PAIRING },
                    onNavigateToTransfers = { selectedTab = AppTab.TRANSFERS }
                )
                AppTab.HISTORY -> HistoryScreen(
                    onCopyClip = onCopyToClipboard
                )
                AppTab.DEVICES -> DevicesScreen(
                    onNavigateToPairing = { selectedTab = AppTab.PAIRING }
                )
                AppTab.PAIRING -> PairingScreen(
                    onPairingSuccess = {
                        selectedTab = AppTab.DEVICES
                    }
                )
                AppTab.TRANSFERS -> TransfersScreen(
                    onSendFile = { /* Launch system file picker */ }
                )
                AppTab.SECURITY -> SecurityScreen(
                    onClearHistory = { /* Clear DB */ },
                    onResetIdentity = { /* Reset keystore keys */ }
                )
            }
        }
    }
}

