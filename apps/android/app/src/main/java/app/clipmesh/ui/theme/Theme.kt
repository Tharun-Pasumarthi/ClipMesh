package app.clipmesh.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

private val DarkColorScheme = darkColorScheme(
    primary = Indigo500,
    secondary = Violet500,
    tertiary = Emerald500,
    background = Slate950,
    surface = Slate900,
    onPrimary = Slate100,
    onSecondary = Slate100,
    onBackground = Slate100,
    onSurface = Slate200,
    surfaceVariant = Slate800,
    onSurfaceVariant = Slate400
)

private val LightColorScheme = lightColorScheme(
    primary = Indigo600,
    secondary = Violet500,
    tertiary = Emerald500,
    background = Color(0xFFF8FAFC),
    surface = Color(0xFFFFFFFF),
    onPrimary = Color(0xFFFFFFFF),
    onSecondary = Color(0xFFFFFFFF),
    onBackground = Slate900,
    onSurface = Slate800,
    surfaceVariant = Slate100,
    onSurfaceVariant = Slate700
)

@Composable
fun ClipMeshTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit
) {
    val colorScheme = if (darkTheme) DarkColorScheme else LightColorScheme
    MaterialTheme(
        colorScheme = colorScheme,
        content = content
    )
}
