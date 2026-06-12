//! Audience command handlers.

use mkt_core::error::Result;
use mkt_core::output::{OutputFormat, format_output};
use mkt_core::provider::MarketingProvider;

use crate::cli::AudienceAction;

/// Execute an audience action.
pub async fn execute(
    action: &AudienceAction,
    provider: &impl MarketingProvider,
    output_format: OutputFormat,
    dry_run: bool,
) -> Result<String> {
    match action {
        AudienceAction::List => {
            let audiences = provider.list_audiences().await?;
            format_output(&audiences, output_format)
        }
        AudienceAction::Create { name, description } => {
            let input = mkt_core::models::CreateAudienceInput {
                name: name.clone(),
                description: description.clone(),
                ..Default::default()
            };
            let audience = provider.create_audience(&input).await?;
            format_output(&[audience], output_format)
        }
        AudienceAction::AddUsers { id, email, phone } => {
            let users = build_users(email, phone);
            if dry_run {
                return Ok(format!(
                    "[dry-run] Would add {} user(s) to audience {id} \
                     (identifiers hashed locally before upload)",
                    users.len()
                ));
            }
            let result = provider
                .add_users_to_audience(&mkt_core::models::AudienceId::from(id.as_str()), &users)
                .await?;
            if matches!(output_format, OutputFormat::Json) {
                Ok(serde_json::to_string_pretty(&result)?)
            } else {
                Ok(format!(
                    "Audience {}: {} received, {} invalid.",
                    result.audience_id, result.num_received, result.num_invalid
                ))
            }
        }
    }
}

/// Build one [`mkt_core::models::AudienceUser`] per identifier flag.
fn build_users(emails: &[String], phones: &[String]) -> Vec<mkt_core::models::AudienceUser> {
    let mut users: Vec<mkt_core::models::AudienceUser> = Vec::new();
    for email in emails {
        users.push(mkt_core::models::AudienceUser {
            email: Some(email.clone()),
            phone: None,
            external_id: None,
        });
    }
    for phone in phones {
        users.push(mkt_core::models::AudienceUser {
            email: None,
            phone: Some(phone.clone()),
            external_id: None,
        });
    }
    users
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_users_one_per_identifier() {
        let users = build_users(
            &["a@example.com".into(), "b@example.com".into()],
            &["+15551234567".into()],
        );
        assert_eq!(users.len(), 3);
        assert_eq!(users[0].email.as_deref(), Some("a@example.com"));
        assert!(users[0].phone.is_none());
        assert_eq!(users[2].phone.as_deref(), Some("+15551234567"));
        assert!(users[2].email.is_none());
    }

    #[test]
    fn build_users_empty_inputs_yield_empty_vec() {
        assert!(build_users(&[], &[]).is_empty());
    }
}
