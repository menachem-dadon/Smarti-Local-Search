# Design system

The supplied `tokens.zip` is the origin of `desktop/src/design-system/tokens.ts`, `system.css` and shared primitives. The supplied Smarti redesign plan is an architectural/design reference; unrelated Smarti chat/provider/workbench features were not imported into this independent search product.

Product CSS consumes shared tokens for colors, typography, spacing, borders, radii, focus, motion and shadows. Theme projection comes from tokens.ts. Language controls physical direction: the navigation rail sits on the right in Hebrew and on the left in English. Paths/code use LTR isolation; document content uses automatic text direction.

Icons are official Tabler React 3.48.0 SVG assets. Semantic roles map centrally in icons.tsx. Direct module imports keep the bundle bounded; SVG geometry is unchanged. The app/installer icon is rendered from official IconFileSearch, using the shared brand palette.

Dialogs use the browser top layer, trap focus, restore focus and provide safe destructive confirmation. The onboarding dialog intentionally cannot be dismissed before scope selection. Menus escape scroll clipping, expose keyboard semantics and close after activation. Motion respects system/user reduced-motion preferences. Focus and forced-color styles remain available.

React tests cover native SVG roles, RTL ownership, menu actions, switches, HTML-safe passages and matching translation keys. Live native layout evidence is separate; see VALIDATION.md.
