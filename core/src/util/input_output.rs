// check-if-email-exists
// Copyright (C) 2018-2022 Reacher

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::time::Duration;

use async_smtp::{ClientSecurity, ClientTlsParameters};
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};

use crate::misc::{MiscDetails, MiscError};
use crate::mx::{MxDetails, MxError};
use crate::smtp::{SmtpDetails, SmtpError, SmtpErrorDesc};
use crate::syntax::SyntaxDetails;

/// Perform the email verification via a specified proxy. The usage of a proxy
/// is optional.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct CheckEmailInputProxy {
	/// Use the specified SOCKS5 proxy host to perform email verification.
	pub host: String,
	/// Use the specified SOCKS5 proxy port to perform email verification.
	pub port: u16,
	/// Username to pass to proxy authentication.
	pub username: Option<String>,
	/// Password to pass to proxy authentication.
	pub password: Option<String>,
}

/// Define how to apply TLS to a SMTP client connection. Will be converted into
/// async_smtp::ClientSecurity.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum SmtpSecurity {
	/// Insecure connection only (for testing purposes).
	None,
	/// Start with insecure connection and use `STARTTLS` when available.
	Opportunistic,
	/// Start with insecure connection and require `STARTTLS`.
	Required,
	/// Use TLS wrapped connection.
	Wrapper,
}

impl Default for SmtpSecurity {
	fn default() -> Self {
		Self::Opportunistic
	}
}

impl SmtpSecurity {
	pub fn to_client_security(self, tls_params: ClientTlsParameters) -> ClientSecurity {
		match self {
			Self::None => ClientSecurity::None,
			Self::Opportunistic => ClientSecurity::Opportunistic(tls_params),
			Self::Required => ClientSecurity::Required(tls_params),
			Self::Wrapper => ClientSecurity::Wrapper(tls_params),
		}
	}
}

/// Builder pattern for the input argument into the main `email_exists`
/// function.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct CheckEmailInput {
	/// The email to validate.
	pub to_email: String,
	/// Email to use in the `MAIL FROM:` SMTP command.
	///
	/// Defaults to "reacher.email@gmail.com", which is an unused addressed
	/// owned by Reacher.
	pub from_email: String,
	/// Name to use in the `EHLO:` SMTP command.
	///
	/// Defaults to "gmail.com" (note: "localhost" is not a FQDN).
	pub hello_name: String,
	/// Perform the email verification via the specified SOCK5 proxy. The usage of a
	/// proxy is optional.
	pub proxy: Option<CheckEmailInputProxy>,
	/// SMTP port to use for email validation. Generally, ports 25, 465, 587
	/// and 2525 are used.
	///
	/// Defaults to 25.
	pub smtp_port: u16,
	/// Add timeout for the SMTP verification step. Set to None if you don't
	/// want to use a timeout.
	///
	/// Defaults to 12s (more than 10s, but when run twice less than 30s).
	pub smtp_timeout: Option<Duration>,
	/// For Yahoo email addresses, use Yahoo's API instead of connecting
	/// directly to their SMTP servers.
	///
	/// Defaults to true.
	pub yahoo_use_api: bool,
	/// For Gmail email addresses, use Gmail's API instead of connecting
	/// directly to their SMTP servers.
	///
	/// Defaults to false.
	pub gmail_use_api: bool,
	/// For Microsoft 365 email addresses, use OneDrive's API instead of
	/// connecting directly to their SMTP servers.
	///
	/// Defaults to false.
	pub microsoft365_use_api: bool,
	// Whether to check if a gravatar image is existing for the given email.
	//
	// Defaults to false.
	pub check_gravatar: bool,
	/// Check if a the email address is present in HaveIBeenPwned API.
	// If the api_key is filled, HaveIBeenPwned API is checked
	pub haveibeenpwned_api_key: Option<String>,
	/// For Hotmail/Outlook email addresses, use a headless navigator
	/// connecting to the password recovery page instead of the SMTP server.
	/// This assumes you have a WebDriver compatible process running, then pass
	/// its endpoint, usually http://localhost:4444. We recommend running
	/// chromedriver (and not geckodriver) as it allows parallel requests.
	///
	/// Defaults to None.
	#[cfg(feature = "headless")]
	pub hotmail_use_headless: Option<String>,
	/// Number of retries of SMTP connections to do.
	///
	/// Defaults to 2 to avoid greylisting.
	pub retries: usize,
	/// How to apply TLS to a SMTP client connection.
	///
	/// Defaults to Opportunistic.
	pub smtp_security: SmtpSecurity,
	/// **IMPORTANT:** This is a beta feature, and might be completely removed,
	/// or moved somewhere else, before the next release.
	///
	/// List of domains to skip when doing an SMTP connection, because we know
	/// they return "unknown". For each string in this list, we check if the MX
	/// record **contains** the string; if yes, we return an error saying the
	/// SMTP verification is skipped.
	///
	/// Related issue: https://github.com/reacherhq/check-if-email-exists/issues/937
	///
	/// ## Example
	///
	/// If you want to skip Zoho emails, it's good enough to ".zoho.com." to
	/// the list. This way it will skip all Zoho emails as their MX records
	/// show "mx{N}.zoho.com.". Simply putting "zoho" might give false
	/// positives if you have an email provider like "mycustomzoho.com".
	///
	/// Defaults to: [""]
	pub skipped_domains: Vec<String>,
}

impl Default for CheckEmailInput {
	fn default() -> Self {
		CheckEmailInput {
			to_email: "".into(),
			from_email: "reacher.email@gmail.com".into(), // Unused, owned by Reacher
			hello_name: "gmail.com".into(),
			#[cfg(feature = "headless")]
			hotmail_use_headless: None,
			proxy: None,
			smtp_port: 25,
			smtp_security: SmtpSecurity::default(),
			smtp_timeout: Some(Duration::from_secs(12)),
			yahoo_use_api: true,
			gmail_use_api: false,
			microsoft365_use_api: false,
			check_gravatar: false,
			haveibeenpwned_api_key: None,
			retries: 2,
			skipped_domains: vec![
				// on @bluewin.ch
				// - mx-v02.bluewin.ch.
				".bluewin.ch.".into(),
				// on @bluewin.ch
				// - mxbw-bluewin-ch.hdb-cs04.ellb.ch.
				"bluewin-ch.".into(),
				// on @gmx.de, @gmx.ch, @gmx.net
				".gmx.net.".into(),
				// on @icloud.com
				".mail.icloud.com.".into(),
				// on @web.de
				".web.de.".into(),
				".zoho.com.".into(),
			],
		}
	}
}

impl CheckEmailInput {
	/// Create a new CheckEmailInput.
	pub fn new(to_email: String) -> CheckEmailInput {
		CheckEmailInput {
			to_email,
			..Default::default()
		}
	}

	/// Set the email to use in the `MAIL FROM:` SMTP command. Defaults to
	/// `user@example.org` if not explicitly set.
	#[deprecated(since = "0.8.24", note = "Please use set_from_email instead")]
	pub fn from_email(&mut self, email: String) -> &mut CheckEmailInput {
		self.from_email = email;
		self
	}

	/// Set the email to use in the `MAIL FROM:` SMTP command. Defaults to
	/// `user@example.org` if not explicitly set.
	pub fn set_from_email(&mut self, email: String) -> &mut CheckEmailInput {
		self.from_email = email;
		self
	}

	/// Set the name to use in the `EHLO:` SMTP command. Defaults to `localhost`
	/// if not explicitly set.
	#[deprecated(since = "0.8.24", note = "Please use set_hello_name instead")]
	pub fn hello_name(&mut self, name: String) -> &mut CheckEmailInput {
		self.hello_name = name;
		self
	}

	/// Set the name to use in the `EHLO:` SMTP command. Defaults to `localhost`
	/// if not explicitly set.
	pub fn set_hello_name(&mut self, name: String) -> &mut CheckEmailInput {
		self.hello_name = name;
		self
	}

	/// Use the specified SOCK5 proxy to perform email verification.
	#[deprecated(since = "0.8.24", note = "Please use set_proxy instead")]
	pub fn proxy(&mut self, proxy_host: String, proxy_port: u16) -> &mut CheckEmailInput {
		self.proxy = Some(CheckEmailInputProxy {
			host: proxy_host,
			port: proxy_port,
			..Default::default()
		});
		self
	}

	/// Use the specified SOCK5 proxy to perform email verification.
	pub fn set_proxy(&mut self, proxy: CheckEmailInputProxy) -> &mut CheckEmailInput {
		self.proxy = Some(proxy);
		self
	}

	/// Set the number of SMTP retries to do.
	pub fn set_retries(&mut self, retries: usize) -> &mut CheckEmailInput {
		self.retries = retries;
		self
	}

	/// Add optional timeout for the SMTP verification step.
	#[deprecated(since = "0.8.24", note = "Please use set_smtp_timeout instead")]
	pub fn smtp_timeout(&mut self, duration: Duration) -> &mut CheckEmailInput {
		self.smtp_timeout = Some(duration);
		self
	}

	/// Change the SMTP port.
	pub fn set_smtp_port(&mut self, port: u16) -> &mut CheckEmailInput {
		self.smtp_port = port;
		self
	}

	/// Set the SMTP client security to use for TLS.
	pub fn set_smtp_security(&mut self, smtp_security: SmtpSecurity) -> &mut CheckEmailInput {
		self.smtp_security = smtp_security;
		self
	}

	/// Add optional timeout for the SMTP verification step. This is the
	/// timeout for _each_ SMTP connection attempt, not for the whole email
	/// verification process.
	pub fn set_smtp_timeout(&mut self, duration: Option<Duration>) -> &mut CheckEmailInput {
		self.smtp_timeout = duration;
		self
	}

	/// Set whether to use Yahoo's API or connecting directly to their SMTP
	/// servers. Defaults to true.
	#[deprecated(since = "0.8.24", note = "Please use set_yahoo_use_api instead")]
	pub fn yahoo_use_api(&mut self, use_api: bool) -> &mut CheckEmailInput {
		self.yahoo_use_api = use_api;
		self
	}

	/// Set whether to use Yahoo's API or connecting directly to their SMTP
	/// servers. Defaults to true.
	pub fn set_yahoo_use_api(&mut self, use_api: bool) -> &mut CheckEmailInput {
		self.yahoo_use_api = use_api;
		self
	}

	/// Set whether to use Gmail's API or connecting directly to their SMTP
	/// servers. Defaults to false.
	pub fn set_gmail_use_api(&mut self, use_api: bool) -> &mut CheckEmailInput {
		self.gmail_use_api = use_api;
		self
	}

	/// Set whether to use Microsoft 365's OneDrive API or connecting directly
	/// to their SMTP servers. Defaults to false.
	pub fn set_microsoft365_use_api(&mut self, use_api: bool) -> &mut CheckEmailInput {
		self.microsoft365_use_api = use_api;
		self
	}

	/// Whether to check if a gravatar image is existing for the given email.
	/// Defaults to false.
	pub fn set_check_gravatar(&mut self, check_gravatar: bool) -> &mut CheckEmailInput {
		self.check_gravatar = check_gravatar;
		self
	}

	/// Whether to haveibeenpwned' API for the given email
	/// check only if the api_key is set
	pub fn set_haveibeenpwned_api_key(&mut self, api_key: Option<String>) -> &mut CheckEmailInput {
		self.haveibeenpwned_api_key = api_key;
		self
	}

	/// Set whether or not to use a headless navigator to navigate to Hotmail's
	/// password recovery page to check if an email exists. If set to
	/// `Some(<endpoint>)`, this endpoint must point to a WebDriver process,
	/// usually listening on http://localhost:4444. Defaults to None.
	#[cfg(feature = "headless")]
	pub fn set_hotmail_use_headless(
		&mut self,
		use_headless: Option<String>,
	) -> &mut CheckEmailInput {
		self.hotmail_use_headless = use_headless;
		self
	}

	/// **IMPORTANT:** This is a beta feature, and might be completely removed,
	/// or moved somewhere else, before the next release.
	///
	/// List of domains to skip when doing an SMTP connection, because we know
	/// they return "unknown".
	pub fn set_skipped_domains(&mut self, domains: Vec<String>) -> &mut CheckEmailInput {
		self.skipped_domains = domains;
		self
	}
}

/// An enum to describe how confident we are that the recipient address is
/// real.
#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Reachable {
	/// The email is safe to send.
	Safe,
	/// The email address appears to exist, but has quality issues that may
	/// result in low engagement or a bounce. Emails are classified as risky
	/// when one of the following happens:
	/// - catch-all email,
	/// - disposable email,
	/// - role-based address,
	/// - full inbox.
	Risky,
	/// Emails that don't exist or are syntactically incorrect. Do not send to
	/// these emails.
	Invalid,
	/// We're unable to get a valid response from the recipient's email server.
	Unknown,
}

/// The result of the [check_email](check_email) function.
#[derive(Debug)]
pub struct CheckEmailOutput {
	/// Input by the user.
	pub input: String,
	pub is_reachable: Reachable,
	/// Misc details about the email address.
	pub misc: Result<MiscDetails, MiscError>,
	/// Details about the MX host.
	pub mx: Result<MxDetails, MxError>,
	/// Details about the SMTP responses of the email.
	pub smtp: Result<SmtpDetails, SmtpError>,
	/// Details about the email address.
	pub syntax: SyntaxDetails,
}

impl Default for CheckEmailOutput {
	fn default() -> Self {
		CheckEmailOutput {
			input: String::default(),
			is_reachable: Reachable::Unknown,
			misc: Ok(MiscDetails::default()),
			mx: Ok(MxDetails::default()),
			smtp: Ok(SmtpDetails::default()),
			syntax: SyntaxDetails::default(),
		}
	}
}

// Implement a custom serialize.
impl Serialize for CheckEmailOutput {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		// This is just used internally to get the nested error field.
		#[derive(Serialize)]
		struct MyError<E> {
			error: E,
			// We add an optional "description" field when relevant, given by
			// the `get_description` on SmtpError.
			#[serde(skip_serializing_if = "Option::is_none")]
			description: Option<SmtpErrorDesc>,
		}

		let mut map = serializer.serialize_map(Some(1))?;
		map.serialize_entry("input", &self.input)?;
		map.serialize_entry("is_reachable", &self.is_reachable)?;
		match &self.misc {
			Ok(t) => map.serialize_entry("misc", &t)?,
			Err(error) => map.serialize_entry(
				"misc",
				&MyError {
					error,
					description: None,
				},
			)?,
		}
		match &self.mx {
			Ok(t) => map.serialize_entry("mx", &t)?,
			Err(error) => map.serialize_entry(
				"mx",
				&MyError {
					error,
					description: None,
				},
			)?,
		}
		match &self.smtp {
			Ok(t) => map.serialize_entry("smtp", &t)?,
			Err(error) => map.serialize_entry(
				"smtp",
				&MyError {
					error,
					description: error.get_description(),
				},
			)?,
		}
		map.serialize_entry("syntax", &self.syntax)?;
		map.end()
	}
}

#[cfg(test)]
mod tests {
	use super::CheckEmailOutput;
	use async_smtp::smtp::response::{Category, Code, Detail, Response, Severity};

	#[test]
	fn should_serialize_correctly() {
		// create a dummy CheckEmailOutput, with a given message as a transient
		// SMTP error.
		fn dummy_response_with_message(m: &str) -> CheckEmailOutput {
			let r = Response::new(
				Code {
					severity: Severity::TransientNegativeCompletion,
					category: Category::MailSystem,
					detail: Detail::Zero,
				},
				vec![m.to_string(), "8BITMIME".to_string(), "SIZE 42".to_string()],
			);

			CheckEmailOutput {
				input: "foo".to_string(),
				is_reachable: super::Reachable::Unknown,
				misc: Ok(super::MiscDetails::default()),
				mx: Ok(super::MxDetails::default()),
				syntax: super::SyntaxDetails::default(),
				smtp: Err(super::SmtpError::SmtpError(r.into())),
			}
		}

		let res = dummy_response_with_message("blacklist");
		let actual = serde_json::to_string(&res).unwrap();
		// Make sure the `description` is present with IpBlacklisted.
		let expected = r#"{"input":"foo","is_reachable":"unknown","misc":{"is_disposable":false,"is_role_account":false,"gravatar_url":null,"haveibeenpwned":null},"mx":{"accepts_mail":false,"records":[]},"smtp":{"error":{"type":"SmtpError","message":"transient: blacklist"},"description":"IpBlacklisted"},"syntax":{"address":null,"domain":"","is_valid_syntax":false,"username":"","normalized_email":null,"suggestion":null}}"#;
		assert_eq!(expected, actual);

		let res =
			dummy_response_with_message("Client host rejected: cannot find your reverse hostname");
		let actual = serde_json::to_string(&res).unwrap();
		// Make sure the `description` is present with NeedsRDNs.
		let expected = r#"{"input":"foo","is_reachable":"unknown","misc":{"is_disposable":false,"is_role_account":false,"gravatar_url":null,"haveibeenpwned":null},"mx":{"accepts_mail":false,"records":[]},"smtp":{"error":{"type":"SmtpError","message":"transient: Client host rejected: cannot find your reverse hostname"},"description":"NeedsRDNS"},"syntax":{"address":null,"domain":"","is_valid_syntax":false,"username":"","normalized_email":null,"suggestion":null}}"#;
		assert_eq!(expected, actual);

		let res = dummy_response_with_message("foobar");
		let actual = serde_json::to_string(&res).unwrap();
		// Make sure the `description` is NOT present.
		let expected = r#"{"input":"foo","is_reachable":"unknown","misc":{"is_disposable":false,"is_role_account":false,"gravatar_url":null,"haveibeenpwned":null},"mx":{"accepts_mail":false,"records":[]},"smtp":{"error":{"type":"SmtpError","message":"transient: foobar"}},"syntax":{"address":null,"domain":"","is_valid_syntax":false,"username":"","normalized_email":null,"suggestion":null}}"#;
		assert_eq!(expected, actual);
	}
}
