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
  ☐ Implement git worktree integration (Each task, once started, should create a temporary worktree in some temperory location [if thats not automatic when you make a worktree], with a button to open file location)
MINOR ADDITIONS
  ☐ Add tabbed sidebar [Instead of showcasing the Agent Chat only in the sidebar (when a task is selected so that the sidebar is active), make it two tabs where the default will be the same Agent Chat view, but the 2nd tab will be a 'Diff' view, that will showcase the files and their diffs, like in the github kinda format for each file within the worktree](Agent Chat / Diffs [only visible in boards 'In Progress' and onward])
  ☐ Add delete board action in kanban view top right (alongside back to proejcts, edit, and add task) so there is a way to delete a board. Make sure there is a confirmation window
  ☐ Add open IDE board action in kanban view top right (alongside back to proejcts, edit, and add task) so vscode (the default IDE) can automatically open to the saved project directory.
  - ☐ Additionally possibly a open Folder directory as well?
  ☐ Add system tray functionality (minimze to system tray)
