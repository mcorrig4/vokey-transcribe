Our mental model is operating in two modes: developer and project manager.

You have the following tool to send me notifications:
.claude

We follow:
1. What to do next?
2. Choose a new feature based on project docs, gh issue and PRs, and user prompting
3. Plan - discuss architecture decisions, walk through though process of expert architect following latest documentation and best practices asking questions at decision points while they walk through building out their final plan
4. create issue feature branch, push a plan doc, create draft PR (linked to any related gh issues)
5. use .claude/bin/ntfy-send  tell me plan is ready with .claude/bin/ntfy-send


---

We got cut off. You had just explored the entire repo after I asked you what should we do next for sprint 7. You asked if option A or B and I wanted option A. Explore again, then we will continue

PR 125 is in UAT and expected to be merged shortly. While I finish testing please start planning for the next task: Issue #72

We should next think about the best way to architect this. Talk through the design and the decisions we should consider given we follow rust and tauri and Linux and OpenAI audio processing best practices.
We should have a sub agent research the web and the project and return all the relevant information for us to plan. 

Give me a detailed discussion on Tauri and rust architecture design. The. Give me a plan for how to build the functionality. Then give me the UI enhancements plan. Then I will review and decide next steps

---


please create an issues doc for me now, following a 2-phase implementation plan, then setup gh issues with a master tracking issue for this new feature sprint and individual issues for all the tasks.


---

get a sub agent to build the next phase of the plan. 

when the initial implementation is done, commit and mark PR not draft. assign @copilot as a reviewer on the PR.  then send me a progress update to ntfy.sh with a link to the PR.

then launch 2 parallel code review sub agents. each agent will do a FULL code review of the PR and overall codebase. we want redundancy in coverage to make sure we don't miss anything. when the agents finish their independent code reviews and report their findings back to you. 

you then check the PR to see if there are updated automated code review commetns from gemini and copilot. 

summarise the findings from all the code reviews. classify each finding as: needs fix, should fix, should defer (create a new gh issue for the feature and label with priority), or don't fix/false positive/wrong. send me ntfy.sh message to say code reviews are done, 

finally, tell me your recommened next steps and I'll discuss what to do next with you



---

let's do issue 76. tell me your full plan highlighting the decisions you made to create an architecture that follows our project patters and latest rust, tauri and react best practices



---

then implement this plan following our expert judgement and careful attention to detail. once done, create a PR. then when the initial implementation is done, commit and mark PR not draft. assign @copilot as a reviewer on the PR.  then send me a progress update to ntfy.sh with a link to the PR.

then launch 2 parallel code review sub agents. each agent will do a FULL code review of the PR and overall codebase. we want redundancy in coverage to make sure we don't miss anything. when the agents finish their independent code reviews and report their findings back to you. 

you then check the PR to see if there are updated automated code review commetns from gemini and copilot. 

summarise the findings from all the code reviews. classify each finding as: needs fix, should fix, should defer (create a new gh issue for the feature and label with priority), or don't fix/false positive/wrong. send me ntfy.sh message to say code reviews are done, 

finally, tell me your recommened next steps and I'll discuss what to do next with you

---

then continue following our workflow to complete all of sprint 7a. do not stop until you've completed and validated all of sprint 7a to the absolute best of your ability.

1. start an issue: describe your plan, research agent(s) for web or repo scouting, then fully describe to me your plan, highlighting every architecture decision you are making to follow rust and tauri and react best practices.
2. create a new feature branch and create a set of issues with tasks and good verification criteria and save with other issue docs
3. commit, push then start implementing
4. use a sub agent to implement everything and test and format and commit
5. create a new PR
6. ask @copilot to review the PR
7. launch two parallel sub agents. each agent should do an individual full code review. they should not specialize but rather give us a redundant checker for all possible bugs, risks, or room for simplification. If you also want to launch 1-2 specialist focused code review sub agents in parallel as well you can do that.
8. let the agents do their thing. this will give our remote gemini and copilot reviews time to process.
9. when the sub agents have returned their feedback, check for any new comments on the PR from automated reviews. consolidate all of the code review findings and then use your expertise and judgement to classify things to fix, defer (gh issue new), or dismiss (false positive etc.). Present this summary report to me, then ntfy.sh message me about process. then tell me your recommended next steps.
10. post a comment on the PR with a starting commit sha for tracking, then assume I approve your recommendations for the fixes.
10.5 ensure worklog is properly updated. make sure all docs are current to the actual spec of the app.
11. once fixes are done, do another sub agent review final pass. commit, and comment your work on the PR.
12. comment "/gemini please review" on the PR.
13. comment with a UAT manual checklist for me to test the PR
14. send me ntfy.sh with link to completed PR
15. start all over! branching off of here, with the optimistic assumption that UAT will pass. determine the next best issue to tackle, explain your plan and reasoning, etc. etc. keep going with this loop until you have completed sprint 7a

BONUS
16. After you have implemented all of sprint 7a following this loop, finally: do one last optimization, simplifications, bug squashing, hardening, and improving pass. first plan out a 3 wave plan for review, fix, rereview/validate of parallel sub agents. the first wave will have multiple agents explore the entire code base, brainstorming different approaches and doing deep code review. when these first 3-5 agents return, use 1-2 sub agents to implement the fixes (or defer gh issues). Do the third wave of 2 agents validating and testing and reviewing once more independently. finally, give a huge report final PR comment with your report and an exhaustive, full UAT testing checklist, broken into logical MECE-type sections chronologically to test the full app. (aka UAT Testing for Dummy's type instructions) formatted as a check list
