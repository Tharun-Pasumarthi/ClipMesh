package app.clipmesh.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.clipmesh.ui.theme.*

data class ClipItem(
    val id: Long,
    val title: String,
    val body: String,
    val kind: String,
    val isPinned: Boolean,
    val origin: String,
    val timestamp: String
)

@Composable
fun HistoryScreen(
    onCopyClip: (String) -> Unit
) {
    var searchQuery by remember { mutableStateOf("") }
    var selectedFilter by remember { mutableStateOf("All") }
    var clips by remember {
        mutableStateOf(
            listOf(
                ClipItem(1, "Repository Link", "https://github.com/clipmesh/clipmesh", "Link", true, "Windows Desktop", "2m ago"),
                ClipItem(2, "Rust Signature Verify", "fn verify_signature(sig: &[u8; 64], msg: &[u8]) -> bool", "Code", false, "Linux Server", "15m ago"),
                ClipItem(3, "Meeting Notes", "Discussed E2EE multi-recipient ratcheting and local SQLite persistence", "Text", false, "This Phone", "1h ago")
            )
        )
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp)
    ) {
        // Search TextField
        OutlinedTextField(
            value = searchQuery,
            onValueChange = { searchQuery = it },
            placeholder = { Text("Search clipboard history...", color = Slate400, fontSize = 13.sp) },
            colors = OutlinedTextFieldDefaults.colors(
                focusedContainerColor = Slate900,
                unfocusedContainerColor = Slate900,
                focusedBorderColor = Indigo500,
                unfocusedBorderColor = Slate800,
                focusedTextColor = Slate100,
                unfocusedTextColor = Slate100
            ),
            shape = RoundedCornerShape(8.dp),
            modifier = Modifier.fillMaxWidth()
        )

        Spacer(modifier = Modifier.height(12.dp))

        // Filter Pills
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            listOf("All", "Text", "Code", "Link").forEach { filter ->
                val isSelected = selectedFilter == filter
                Box(
                    modifier = Modifier
                        .clip(RoundedCornerShape(16.dp))
                        .background(if (isSelected) Indigo600 else Slate800)
                        .clickable { selectedFilter = filter }
                        .padding(horizontal = 14.dp, vertical = 6.dp)
                ) {
                    Text(
                        text = filter,
                        color = if (isSelected) Slate100 else Slate400,
                        fontSize = 12.sp,
                        fontWeight = if (isSelected) FontWeight.SemiBold else FontWeight.Normal
                    )
                }
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        // Clips List
        val filteredClips = clips.filter {
            (selectedFilter == "All" || it.kind.equals(selectedFilter, ignoreCase = true)) &&
            (searchQuery.isEmpty() || it.body.contains(searchQuery, ignoreCase = true) || it.title.contains(searchQuery, ignoreCase = true))
        }

        if (filteredClips.isEmpty()) {
            Box(
                modifier = Modifier.fillMaxSize(),
                contentAlignment = Alignment.Center
            ) {
                Text("No clips found", color = Slate400, fontSize = 14.sp)
            }
        } else {
            LazyColumn(
                verticalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                items(filteredClips, key = { it.id }) { clip ->
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
                                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                    Box(
                                        modifier = Modifier
                                            .clip(RoundedCornerShape(4.dp))
                                            .background(Slate800)
                                            .padding(horizontal = 6.dp, vertical = 2.dp)
                                    ) {
                                        Text(text = clip.kind.uppercase(), color = Slate200, fontSize = 9.sp, fontWeight = FontWeight.Bold)
                                    }
                                    if (clip.isPinned) {
                                        Box(
                                            modifier = Modifier
                                                .clip(RoundedCornerShape(4.dp))
                                                .background(Amber500.copy(alpha = 0.2f))
                                                .padding(horizontal = 6.dp, vertical = 2.dp)
                                        ) {
                                            Text(text = "PINNED", color = Amber500, fontSize = 9.sp, fontWeight = FontWeight.Bold)
                                        }
                                    }
                                }
                                Text(text = clip.timestamp, color = Slate400, fontSize = 11.sp)
                            }

                            Spacer(modifier = Modifier.height(8.dp))
                            Text(
                                text = clip.body,
                                color = Slate100,
                                fontSize = 13.sp,
                                maxLines = 4
                            )
                            Spacer(modifier = Modifier.height(10.dp))

                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Text(
                                    text = "From: ${clip.origin}",
                                    color = Slate400,
                                    fontSize = 11.sp
                                )

                                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                    TextButton(
                                        onClick = {
                                            clips = clips.map {
                                                if (it.id == clip.id) it.copy(isPinned = !it.isPinned) else it
                                            }
                                        },
                                        contentPadding = PaddingValues(horizontal = 8.dp, vertical = 2.dp)
                                    ) {
                                        Text(
                                            if (clip.isPinned) "Unpin" else "Pin",
                                            color = if (clip.isPinned) Amber500 else Slate400,
                                            fontSize = 12.sp
                                        )
                                    }

                                    Button(
                                        onClick = { onCopyClip(clip.body) },
                                        colors = ButtonDefaults.buttonColors(containerColor = Indigo600),
                                        shape = RoundedCornerShape(6.dp),
                                        contentPadding = PaddingValues(horizontal = 12.dp, vertical = 2.dp)
                                    ) {
                                        Text("Copy", fontSize = 12.sp)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

