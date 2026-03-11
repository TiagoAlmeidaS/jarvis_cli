use jarvis_protocol::protocol::IssueResolverRequest;

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedIssueResolver {
    pub github_pat: String,
    pub owner: String,
    pub repo: String,
    pub issue_number: Option<u64>,
    pub prompt: String,
    pub user_facing_hint: String,
}

pub fn resolve_issue_resolver_request(
    request: IssueResolverRequest,
    github_pat: String,
) -> anyhow::Result<ResolvedIssueResolver> {
    let IssueResolverRequest {
        owner,
        repo,
        issue_number,
    } = request;

    let owner = owner.trim().to_string();
    let repo = repo.trim().to_string();

    if owner.is_empty() {
        anyhow::bail!("Owner cannot be empty");
    }
    if repo.is_empty() {
        anyhow::bail!("Repo cannot be empty");
    }

    let prompt = if let Some(issue_num) = issue_number {
        format!(
            "Resolve GitHub issue #{} in the repository {}/{}.\n\n\
            Use the autonomous issue resolver to:\n\
            1. Analyze the issue using LLM\n\
            2. Create an implementation plan\n\
            3. Execute the implementation\n\
            4. Create a pull request with the fix",
            issue_num, owner, repo
        )
    } else {
        format!(
            "Resolve GitHub issues in the repository {}/{} that are labeled for autonomous resolution.\n\n\
            Use the autonomous issue resolver to:\n\
            1. Scan for issues with appropriate labels\n\
            2. Analyze the issue using LLM\n\
            3. Create an implementation plan\n\
            4. Execute the implementation\n\
            5. Create a pull request with the fix",
            owner, repo
        )
    };

    let user_facing_hint = if let Some(num) = issue_number {
        format!("issue #{num} in {owner}/{repo}")
    } else {
        format!("issues in {owner}/{repo}")
    };

    Ok(ResolvedIssueResolver {
        github_pat,
        owner,
        repo,
        issue_number,
        prompt,
        user_facing_hint,
    })
}
