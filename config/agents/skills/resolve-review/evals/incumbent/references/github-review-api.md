# GitHub Review-Thread API Recipes

Use these commands only after resolving the target repository and PR. Replace
`OWNER`, `REPO`, `NUMBER`, and IDs with values from the PR metadata; for a fork,
`OWNER/REPO` is the base repository that owns the pull request.

## PR Metadata

```sh
gh pr view <number-or-url> --json \
  number,url,state,headRefName,headRefOid,headRepositoryOwner,isCrossRepository
git status --short
git rev-parse HEAD
```

Do not assume `git fetch` synchronized the checkout. Compare the two OIDs.

## All Review Threads

`--paginate` supplies `endCursor`; `--slurp` returns all page objects in one
JSON array.

```sh
gh api graphql --paginate --slurp \
  -F owner=OWNER -F repo=REPO -F pr=NUMBER \
  -f query='query($owner:String!,$repo:String!,$pr:Int!,$endCursor:String){
    repository(owner:$owner,name:$repo){pullRequest(number:$pr){
      reviewThreads(first:100,after:$endCursor){
        nodes{
          id isResolved isOutdated path line originalLine
          viewerCanReply viewerCanResolve
          comments(first:100){
            totalCount
            nodes{id author{login} body createdAt url}
          }
        }
        pageInfo{hasNextPage endCursor}
      }
    }}
  }'
```

Filter for `isResolved: false`. If a thread's `comments.totalCount` exceeds the
returned node count, paginate that thread's comments before deciding; the latest
discussion may supersede the root comment.

## Reply to a Thread

```sh
gh api graphql \
  -f query='mutation($thread:ID!,$body:String!){
    addPullRequestReviewThreadReply(input:{
      pullRequestReviewThreadId:$thread,
      body:$body
    }){comment{id url}}
  }' \
  -f thread=THREAD_ID \
  -f body='Addressed in COMMIT: concise outcome and verification.'
```

Use a factual reply. Do not say “fixed” when the outcome is disagreement,
deferral, incomplete validation, or a request for clarification.

## Resolve a Satisfied Thread

```sh
gh api graphql \
  -f query='mutation($thread:ID!){
    resolveReviewThread(input:{threadId:$thread}){
      thread{id isResolved}
    }
  }' \
  -f thread=THREAD_ID
```

After replying or resolving, run the paginated query again. Treat the returned
state, not a successful command exit alone, as the completion check.
