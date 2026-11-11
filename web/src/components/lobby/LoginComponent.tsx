// OtherComponents/LoginComponent.razor: log in, create an account, request a password reset, set
// a new password. Passwords are sent as Support.GetHash (state/connection.ts, hashPassword).
import { useEffect, useRef, useState } from "react";
import { hashPassword } from "../../state/connection";
import { useLobbyClient } from "../../lobby/context";
import { isNullOrEmpty, isNullOrWhiteSpace, netTrimLength, validName } from "../../lobby/text";

type State = "Login" | "NewAccount" | "RequestPasswordReset" | "EnterNewPassword" | "Loading";

export function LoginComponent() {
  const client = useLobbyClient();
  const [state, setState] = useState<State>("Login");
  const focusField = useRef<HTMLInputElement>(null);

  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [keepSignedIn, setKeepSignedIn] = useState(true);
  const [loginMessage, setLoginMessage] = useState<string | null>(null);

  const [newUsername, setNewUsername] = useState<string | null>(null);
  const [newPlayerName, setNewPlayerName] = useState<string | null>(null);
  const [newPassword, setNewPassword] = useState<string | null>(null);
  const [newPasswordConfirm, setNewPasswordConfirm] = useState<string | null>(null);
  const [newEmail, setNewEmail] = useState<string | null>(null);
  const [newAccountMessage, setNewAccountMessage] = useState<string | null>(null);

  const [resetUsernameOrEmail, setResetUsernameOrEmail] = useState("");
  const [passwordResetMessage, setPasswordResetMessage] = useState<string | null>(null);

  const [resetToken, setResetToken] = useState("");
  const [resetUsername, setResetUsername] = useState("");
  const [resetPassword, setResetPassword] = useState("");
  const [resetPasswordConfirm, setResetPasswordConfirm] = useState("");
  const [resetPasswordMessage, setResetPasswordMessage] = useState<string | null>(null);

  // OnAfterRenderAsync(firstRender): focus the first field.
  useEffect(() => { focusField.current?.focus(); }, []);

  const clearAllInputs = () => {
    setUsername(""); setPassword(""); setLoginMessage(null);
    setNewUsername(null); setNewPlayerName(null); setNewPassword(null); setNewPasswordConfirm(null); setNewEmail(null); setNewAccountMessage(null);
    setResetUsernameOrEmail(""); setPasswordResetMessage(null);
    setResetToken(""); setResetUsername(""); setResetPassword(""); setResetPasswordConfirm(""); setResetPasswordMessage(null);
  };

  const validateNewAccount = (): boolean => {
    // ValidateNewAccount, with its fall-through: the later checks overwrite the earlier ones.
    let message: string | null = null;
    const nameError = validName(newUsername);
    if (!isNullOrEmpty(nameError)) message = nameError;
    // A field nobody typed in is null in C#, and null passes both "== null ||" length checks.
    if (newUsername == null || netTrimLength(newUsername) <= 3) message = "Username must be more than 3 characters";
    if (newUsername == null || netTrimLength(newUsername) > 40) message = "Username must be 40 characters or less";
    if (newPlayerName == null || netTrimLength(newPlayerName) <= 3) message = "Player name must be more than 3 characters";
    if (newPlayerName == null || netTrimLength(newPlayerName) > 40) message = "Player name must be 40 characters or less";
    else if (isNullOrEmpty(newPassword)) message = "Enter a password";
    else if (newPassword !== newPasswordConfirm) message = "Passwords do not match";
    else if (!isNullOrWhiteSpace(newEmail) && (!newEmail!.includes("@") || !newEmail!.includes("."))) message = "Enter a valid e-mail address, or leave it empty";
    else message = null;
    setNewAccountMessage(message);
    return message == null;
  };

  const login = async () => {
    if (isNullOrEmpty(username) || isNullOrEmpty(password)) return;
    setState("Loading");
    const r = await client.requestLogin(username, await hashPassword(password));
    if (r.Success) {
      client.rememberLogin(keepSignedIn);
      clearAllInputs();
    } else {
      setLoginMessage(client.lobby.describeError(r.Error));
      setState("Login");
    }
  };

  const newAccount = async () => {
    if (!validateNewAccount()) return;
    setState("Loading");
    const r = await client.requestCreateUser(newUsername!, await hashPassword(newPassword!), newEmail, newPlayerName!);
    if (r.Success) {
      client.rememberLogin(keepSignedIn);
      clearAllInputs();
      setState("Login");
    } else {
      setNewAccountMessage(client.lobby.describeError(r.Error));
      setState("NewAccount");
    }
  };

  const passwordReset = async () => {
    if (isNullOrEmpty(resetUsernameOrEmail)) {
      setPasswordResetMessage("Enter your username or e-mail address");
      return;
    }
    setPasswordResetMessage(null);
    setState("Loading");
    const r = await client.requestPasswordReset(resetUsernameOrEmail);
    if (r.Success) {
      clearAllInputs();
      setState("EnterNewPassword");
    } else {
      setPasswordResetMessage(client.lobby.describeError(r.Error));
      setState("RequestPasswordReset");
    }
  };

  const newPasswordSubmit = async () => {
    let message: string | null = null;
    if (isNullOrEmpty(resetToken)) message = "Enter the reset token sent to your e-mail address";
    else if (isNullOrEmpty(resetPassword)) message = "Enter a password";
    else if (resetPassword !== resetPasswordConfirm) message = "Passwords do not match";
    setResetPasswordMessage(message);
    if (message != null) return;
    setState("Loading");
    const r = await client.requestSetPassword(resetUsername, resetToken, await hashPassword(resetPassword));
    if (r.Success) {
      clearAllInputs();
      setState("Login");
    } else {
      setResetPasswordMessage(client.lobby.describeError(r.Error));
      setState("EnterNewPassword");
    }
  };

  const enter = (action: () => void) => (e: React.KeyboardEvent) => {
    if (e.code === "Enter" || e.code === "NumpadEnter") action();
  };

  // OnUserNameChanged (@onchange of the user name): the player name defaults to it.
  const onUserNameChanged = () => {
    if (isNullOrEmpty(newPlayerName)) setNewPlayerName(newUsername);
  };

  if (state === "Loading") {
    return <img src="./art/worm.gif" width={256} style={{ filter: "drop-shadow(1px 1px 1px black)" }} alt="loading..." />;
  }

  const active = (s: State) => (s === state ? "active" : "");
  const tab = (s: State, text: string, current = false) => (
    <li className="nav-item">
      <button className={`nav-link ${active(s)}`} aria-current={current ? "page" : undefined} onClick={() => setState(s)}>{text}</button>
    </li>
  );

  return (
    <div className="card mb-2 shadow-lg" style={{ animation: "fadeIn 3s" }}>
      <div className="card-header">Welcome to treachery.online!</div>
      <div className="card-body">
        <ul className="nav nav-tabs mb-2">
          {tab("Login", "Login", true)}
          {tab("NewAccount", "Create account")}
          {tab("RequestPasswordReset", "Forgot password?")}
          {tab("EnterNewPassword", "Set new password")}
        </ul>

        {state === "Login" && (
          <>
            <div>Please enter your user name and password</div>
            <input ref={focusField} className="form-control mt-1 mt-1" value={username} onChange={(e) => setUsername(e.target.value)} maxLength={40} placeholder="User name" onKeyUp={enter(login)} />
            <input type="password" className="form-control mt-1" value={password} onChange={(e) => setPassword(e.target.value)} maxLength={80} placeholder="Password" onKeyUp={enter(login)} />
            <div className="form-check mt-2">
              <input id="keepSignedIn" className="form-check-input" type="checkbox" checked={keepSignedIn} onChange={(e) => setKeepSignedIn(e.target.checked)} />
              <label htmlFor="keepSignedIn" className="form-check-label">Keep me signed in on this device</label>
            </div>
            <div className="text-danger small">{loginMessage}</div>
            <button className="btn btn-primary mt-1" onClick={login}>Log in</button>
          </>
        )}
        {state === "NewAccount" && (
          <>
            <input ref={focusField} className="form-control mt-1" value={newUsername ?? ""} onChange={(e) => setNewUsername(e.target.value)} onBlur={onUserNameChanged} maxLength={40} placeholder="User name (used to log in)" onKeyUp={enter(newAccount)} />
            <input type="password" className="form-control mt-1" value={newPassword ?? ""} onChange={(e) => setNewPassword(e.target.value)} maxLength={80} placeholder="Password" autoComplete="new-password" onKeyUp={enter(newAccount)} />
            <input type="password" className="form-control mt-1" value={newPasswordConfirm ?? ""} onChange={(e) => setNewPasswordConfirm(e.target.value)} maxLength={80} placeholder="Repeat password" autoComplete="new-password" onKeyUp={enter(newAccount)} />
            <input className="form-control mt-1" value={newEmail ?? ""} onChange={(e) => setNewEmail(e.target.value)} maxLength={80} placeholder="E-mail address (optional, needed only to reset your password)" onKeyUp={enter(newAccount)} />
            <input className="form-control mt-1" value={newPlayerName ?? ""} onChange={(e) => setNewPlayerName(e.target.value)} maxLength={80} placeholder="Player name (visible to other players)" onKeyUp={enter(newAccount)} />
            <div className="text-danger small">{newAccountMessage}</div>
            <button className="btn btn-primary mt-1" onClick={newAccount}>Create account</button>
          </>
        )}
        {state === "RequestPasswordReset" && (
          <>
            <div>Enter the username or e-mail address associated to your account to receive a password reset token</div>
            <input ref={focusField} className="form-control mt-1" value={resetUsernameOrEmail} onChange={(e) => setResetUsernameOrEmail(e.target.value)} maxLength={80} placeholder="Username or e-mail address" onKeyUp={enter(passwordReset)} />
            <div className="text-danger small">{passwordResetMessage}</div>
            <button className="btn btn-primary mt-1" onClick={passwordReset}>Request password reset</button>
          </>
        )}
        {state === "EnterNewPassword" && (
          <>
            <div>Enter the password reset token you received by email, your username and your new password</div>
            <input ref={focusField} className="form-control mt-1" value={resetToken} onChange={(e) => setResetToken(e.target.value)} maxLength={80} placeholder="Reset token" onKeyUp={enter(newPasswordSubmit)} />
            <input className="form-control mt-1" value={resetUsername} onChange={(e) => setResetUsername(e.target.value)} maxLength={40} placeholder="User name" onKeyUp={enter(newPasswordSubmit)} />
            <input type="password" className="form-control mt-1" value={resetPassword} onChange={(e) => setResetPassword(e.target.value)} maxLength={80} placeholder="Password" autoComplete="new-password" onKeyUp={enter(newPasswordSubmit)} />
            <input type="password" className="form-control mt-1" value={resetPasswordConfirm} onChange={(e) => setResetPasswordConfirm(e.target.value)} maxLength={80} placeholder="Repeat password" autoComplete="new-password" onKeyUp={enter(newPasswordSubmit)} />
            <div className="text-danger small">{resetPasswordMessage}</div>
            <button className="btn btn-primary mt-1" onClick={newPasswordSubmit}>Set new password</button>
          </>
        )}
      </div>
      <div className="card-footer">
        <div className="row">
          <span className="col">Version {client.lobby.data.latestVersion} (October 2025)</span>
        </div>
      </div>
    </div>
  );
}
