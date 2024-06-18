
# Pushes HEAD + latest tag atomically
[no-exit-message]
atomic-push-with-tags TAG=`git describe --tags --abbrev=0`:
    git push --atomic origin main {{TAG}}

[no-exit-message]
annotated-tag-version VERSION MSG:
    git tag -a {{VERSION}} -m "{{MSG}}"
