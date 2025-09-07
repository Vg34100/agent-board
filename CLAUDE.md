Always start by taking a look at the two README.md files and the rest of the docs/ documentation to understand the goal of the project.
Remember to occasionally update the docs to reflect newest additions or newest hurdles.

Please make sure to take a very thorough git commiting and branchding methodology
Like creating commits for most addtions in the feat/fix/doc format. Do more incremental commits as we go and can test that we didn't break anything. Don't ma/ke the commits contain the generated with claude code or co authored by cladue stuff.
Creating feature commits for most major additions like feature: project modal creation. If you think the feature is complete, be sure to create a 'pull request' (can just be a summary if github isn't linked) and switch the branch back to main for me to test.

Please make sure to continually check 'cargo check' for compilation errors as we go on.

Last created to-dos
  ✓ Create and connect ProjectModal component (Project must be set to a folder location with a .git folder set up) - COMPLETED
  ✓ Implement data persistence with localStorage - COMPLETED
MAJOR GOALS:
  ☐ Implement git worktree integration (Each task, once started, should create a temporary worktree in some temperory location [if thats not automatic when you make a worktree, im not sure], with a button to open file location [within the side bar somewhere])
  ☐ Implement code agent workflow, (removing the example coding agents). so when the start button is pressed after the temporary worktree is completed. spawn a claude code process that will start in the temp worktree and be passed in the task title and task description as its first message. starting to populate the coding agent side bar view with the first coding agen instance at the current time, and showcasing the coding agent return data. like
  (user message with blue person icon) (gray icon for any system messages like 'system initialzed with model') (green like robot icon for messages from the agent) (yellow eye icon for reading a file, like 'README.md') (red pen/paper icon for editing files by the agent like 'README.md' with a drop down diff block underneath with the exact + + lines or whatever and also like the +2 -0 in the header of the diff block)
MINOR ADDITIONS
  ☐ Add tabbed sidebar [Instead of showcasing the Agent Chat only in the sidebar (when a task is selected so that the sidebar is active), make it two tabs where the default will be the same Agent Chat view, but the 2nd tab will be a 'Diff' view, that will showcase the files and their diffs, like in the github kinda format for each file within the worktree](Agent Chat / Diffs [only visible in boards 'In Progress' and onward])
  ☐ Add delete board action in kanban view top right (alongside back to proejcts, edit, and add task) so there is a way to delete a board. Make sure there is a confirmation window
  ☐ Add open IDE board action in kanban view top right (alongside back to proejcts, edit, and add task) so vscode (the default IDE) can automatically open to the saved project directory.
  - ☐ Additionally possibly a open Folder directory as well?
  ☐ Add system tray functionality (minimze to system tray)
