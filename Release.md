Guide to release:

 - [ ] Verify everything is committed 
 - [ ] update the sqlx cache: cargo sqlx prepare
 - [ ] commit the update
 - [ ] push to github and teilgedanken
 - [ ] create release draft tag: https://github.com/enaut/pslink/releases
 - [ ] check `git log --pretty=format:'* %s' --abbrev-commit` for changes and selectively include into changelist.
 - [ ] verify everything is ready for publishing using:
 
    ```
    SQLX_OFFLINE=1 cargo publish --dry-run
    ```

 - [ ] make draft a release

 - [ ] publish 
 
    ```SQLX_OFFLINE=1 cargo publish```