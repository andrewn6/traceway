type ResendPayload = {
  to: string;
  subject: string;
  text: string;
  html: string;
};

function readEnv(name: string): string {
  const raw = process.env[name]?.trim() ?? "";
  if (!raw) return "";
  if ((raw.startsWith('"') && raw.endsWith('"')) || (raw.startsWith("'") && raw.endsWith("'"))) {
    return raw.slice(1, -1).trim();
  }
  return raw;
}

function appUrl(): string {
  return readEnv("APP_URL") || "http://localhost:5173";
}

async function sendResendEmail(payload: ResendPayload): Promise<void> {
  const apiKey = readEnv("RESEND_API_KEY");
  const from = readEnv("RESEND_FROM") || "Traceway <onboarding@resend.dev>";
  if (!apiKey) {
    throw new Error("RESEND_API_KEY is not configured");
  }

  const res = await fetch("https://api.resend.com/emails", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      from,
      to: [payload.to],
      subject: payload.subject,
      text: payload.text,
      html: payload.html,
    }),
  });

  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(`Resend API ${res.status}${body ? `: ${body.slice(0, 400)}` : ""}`);
  }
}

export async function sendInviteEmail(email: string, token: string, inviterName?: string): Promise<void> {
  const link = `${appUrl()}/accept-invite?token=${encodeURIComponent(token)}`;
  const who = inviterName?.trim() ? `${inviterName} invited you to Traceway` : "You were invited to Traceway";
  const subject = "You're invited to join Traceway";
  const text = `${who}.\n\nAccept invite: ${link}\n\nThis invite expires in 7 days.`;
  const html = `<p>${who}.</p><p><a href="${link}">Accept invite</a></p><p>This invite expires in 7 days.</p>`;

  await sendResendEmail({ to: email, subject, text, html });
}

export async function sendPasswordResetEmail(email: string, token: string): Promise<void> {
  const link = `${appUrl()}/reset-password?token=${encodeURIComponent(token)}`;
  const subject = "Reset your Traceway password";
  const text = `Reset your password: ${link}\n\nThis link expires in 1 hour.`;
  const html = `<p>Reset your Traceway password:</p><p><a href="${link}">Reset password</a></p><p>This link expires in 1 hour.</p>`;

  await sendResendEmail({ to: email, subject, text, html });
}
