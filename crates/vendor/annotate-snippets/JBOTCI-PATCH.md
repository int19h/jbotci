# Bounded diagnostic rendering

This copy comes from annotate-snippets 0.12.16 on crates.io.
The existing license files apply.

The patch adds `Renderer::render_into` for a caller that supplies a bounded writer.
The layout buffer refuses excess text before it grows.
The caller also bounds the final UTF-8 output, which includes style codes and line breaks.
The original `Renderer::render` keeps its output and its unrestricted limit.

The shared server policy uses this entry point for diagnostics in issue #945.
A bound on the final capture buffer cannot control an earlier allocation inside the diagnostic renderer.
