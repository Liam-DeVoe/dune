// Client/Icons (UserIcon, BotIcon, HostIcon) and the svg labels of GenericComponents (PlayIcon,
// TrashIcon, CancelIcon).

function iconStyle(onClick?: () => void) {
  return onClick ? { cursor: "pointer" } : undefined;
}

export function UserIcon({ size = 20, title = "User", marginClass = "m-1", onClick }: { size?: number; title?: string; marginClass?: string; onClick?: () => void }) {
  // UserIcon.razor draws at 16 px whatever its Size.
  void size;
  return (
    <span className={marginClass} onClick={onClick} style={iconStyle(onClick)} title={title}>
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" className="bi bi-person" viewBox="0 0 16 16">
        <path d="M8 8a3 3 0 1 0 0-6 3 3 0 0 0 0 6m2-3a2 2 0 1 1-4 0 2 2 0 0 1 4 0m4 8c0 1-1 1-1 1H3s-1 0-1-1 1-4 6-4 6 3 6 4m-1-.004c-.001-.246-.154-.986-.832-1.664C11.516 10.68 10.289 10 8 10s-3.516.68-4.168 1.332c-.678.678-.83 1.418-.832 1.664z" />
      </svg>
    </span>
  );
}

export function BotIcon({ size = 20, title = "Bot", marginClass = "m-1", onClick }: { size?: number; title?: string; marginClass?: string; onClick?: () => void }) {
  return (
    <span className={marginClass} onClick={onClick} style={iconStyle(onClick)} title={title}>
      <svg xmlns="http://www.w3.org/2000/svg" width={size} height={size} fill="currentColor" className="bi bi-cpu" viewBox="0 0 16 16">
        <path d="M5 0a.5.5 0 0 1 .5.5V2h1V.5a.5.5 0 0 1 1 0V2h1V.5a.5.5 0 0 1 1 0V2h1V.5a.5.5 0 0 1 1 0V2A2.5 2.5 0 0 1 14 4.5h1.5a.5.5 0 0 1 0 1H14v1h1.5a.5.5 0 0 1 0 1H14v1h1.5a.5.5 0 0 1 0 1H14v1h1.5a.5.5 0 0 1 0 1H14a2.5 2.5 0 0 1-2.5 2.5v1.5a.5.5 0 0 1-1 0V14h-1v1.5a.5.5 0 0 1-1 0V14h-1v1.5a.5.5 0 0 1-1 0V14h-1v1.5a.5.5 0 0 1-1 0V14A2.5 2.5 0 0 1 2 11.5H.5a.5.5 0 0 1 0-1H2v-1H.5a.5.5 0 0 1 0-1H2v-1H.5a.5.5 0 0 1 0-1H2v-1H.5a.5.5 0 0 1 0-1H2A2.5 2.5 0 0 1 4.5 2V.5A.5.5 0 0 1 5 0m-.5 3A1.5 1.5 0 0 0 3 4.5v7A1.5 1.5 0 0 0 4.5 13h7a1.5 1.5 0 0 0 1.5-1.5v-7A1.5 1.5 0 0 0 11.5 3zM5 6.5A1.5 1.5 0 0 1 6.5 5h3A1.5 1.5 0 0 1 11 6.5v3A1.5 1.5 0 0 1 9.5 11h-3A1.5 1.5 0 0 1 5 9.5zM6.5 6a.5.5 0 0 0-.5.5v3a.5.5 0 0 0 .5.5h3a.5.5 0 0 0 .5-.5v-3a.5.5 0 0 0-.5-.5z" />
      </svg>
    </span>
  );
}

export function HostIcon({ size = 20, fill = "currentColor", marginClass = "m-1", onClick }: { size?: number; fill?: string; marginClass?: string; onClick?: () => void }) {
  return (
    <span className={marginClass} onClick={onClick} style={iconStyle(onClick)} title="Host">
      <svg xmlns="http://www.w3.org/2000/svg" width={size} height={size} fill={fill} className="bi bi-award-fill" viewBox="0 0 16 16">
        <path d="m8 0 1.669.864 1.858.282.842 1.68 1.337 1.32L13.4 6l.306 1.854-1.337 1.32-.842 1.68-1.858.282L8 12l-1.669-.864-1.858-.282-.842-1.68-1.337-1.32L2.6 6l-.306-1.854 1.337-1.32.842-1.68L6.331.864z" />
        <path d="M4 11.794V16l4-1 4 1v-4.206l-2.018.306L8 13.126 6.018 12.1z" />
      </svg>
    </span>
  );
}

export function PlayIcon() {
  return (
    <svg width="1.2em" height="1.2em" viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg" aria-label="Resume" style={{ verticalAlign: "-0.2em" }}>
      <path fill="currentColor" d="M4.5 2.6v10.8a.6.6 0 0 0 .9.5l8.6-5.4a.6.6 0 0 0 0-1L5.4 2.1a.6.6 0 0 0-.9.5" />
    </svg>
  );
}

export function TrashIcon() {
  return (
    <svg width="1.2em" height="1.2em" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Delete" style={{ verticalAlign: "-0.2em" }}>
      <g stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
        <path d="M2.5 4h11" />
        <path d="M6 4V2.5h4V4" />
        <path d="M3.8 4l.7 9.5h7l.7-9.5" />
        <path d="M6.6 6.5v4.5M9.4 6.5v4.5" />
      </g>
    </svg>
  );
}

export function CancelIcon() {
  return (
    <svg width="1em" height="1em" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Cancel" style={{ verticalAlign: "-0.125em" }}>
      <path d="M3.5 3.5L12.5 12.5M12.5 3.5L3.5 12.5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
    </svg>
  );
}
