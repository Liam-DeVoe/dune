// GenericComponents/CollapseButton.razor: the chevron at the right of a collapsible card header.
// The header itself toggles; the chevron only shows the state.
export function CollapseButton({ collapsed }: { collapsed: boolean }) {
  return (
    <div className="btn btn-link p-0 float-end" style={{ display: "inline-block" }}>
      {collapsed
        ? (
          <svg width="16" height="16" viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M13.825 7.15833L10 10.975L6.175 7.15833L5 8.33332L10 13.3333L15 8.33332L13.825 7.15833Z" fill="currentColor" />
          </svg>
        )
        : (
          <svg width="16" height="16" viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M10 6.66669L5 11.6667L6.175 12.8417L10 9.02502L13.825 12.8417L15 11.6667L10 6.66669Z" fill="currentColor" />
          </svg>
        )}
    </div>
  );
}
