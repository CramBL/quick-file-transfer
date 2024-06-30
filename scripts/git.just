
# Pushes HEAD + latest tag atomically
[group("Git"), no-exit-message]
atomic-push-with-tags BRANCH=`git branch --show-current` TAG=`git describe --tags --abbrev=0`:
    git push --atomic origin {{BRANCH}} {{TAG}}

[group("Git"), no-exit-message]
annotated-tag-version VERSION MSG:
    git tag -a {{VERSION}} -m "{{MSG}}"
