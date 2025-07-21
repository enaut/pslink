# Generated translation template by dioxus-i18n-collect
# Contains 83 translation keys

# Source: ./ui/src/users/database_export.rs:81
database-export-button-clear = Clear

# Source: ./ui/src/users/database_export.rs:73
database-export-button-download = 📥 Download Database

# Source: ./ui/src/users/database_export.rs:115
database-export-button-export = Export Database

# Source: ./ui/src/users/database_export.rs:108
database-export-button-exporting = Exporting...

# Source: ./ui/src/users/database_export.rs:28
database-export-error-empty-secret = Please enter the export secret

# Source: ./ui/src/users/database_export.rs:42
database-export-error-failed = Export failed

# Source: ./ui/src/users/database_export.rs:99
database-export-help-secret = Enter the secret configured in PSLINK_DATA_DOWNLOAD_SECRET environment variable

# Source: ./ui/src/users/database_export.rs:88
database-export-label-secret = Export Secret

# Placeholder for export secret input
# Source: ./ui/src/users/database_export.rs:88
# UNUSED: This key is no longer used in the code
database-export-placeholder-secret = Enter the export secret

# Source: ./ui/src/users/database_export.rs:66
database-export-ready = Database export ready!

# Source: ./ui/src/users/database_export.rs:67
# Parameters: $size
database-export-size = Size: {$size} bytes

# Source: ./ui/src/users/database_export.rs:54
database-export-subtitle = Export the SQLite database for backup purposes

# Source: ./ui/src/users/database_export.rs:36
database-export-success-message = Database exported successfully! Click the download link below.

# Source: ./ui/src/users/database_export.rs:53
database-export-title = Database Export

# Source: ./ui/src/navbar.rs:160
demo-warning = This instance is not created with a persistent storage. So all the links and useraccounts are lost on restart.

# Source: ./ui/src/login.rs:112
# Parameters: $error
failed-login = Username and password did not match please try again.

# Source: ./ui/src/login.rs:40
headline-login = Login

# The menu entry for language selection
# Source: ./ui/src/navbar.rs:86
language = Language Selection

# Button text for confirming link deletion
# Source: ./ui/src/links/link_edit.rs:253
link-edit-button-confirm-delete = Actually delete this link

# Button text for creating a new link
# Source: ./ui/src/links/link_edit.rs:198
link-edit-button-create = Create link

# Button text for deleting a link
# Source: ./ui/src/links/link_edit.rs:209
link-edit-button-delete = Delete link

# Button text for modifying a link
# Source: ./ui/src/links/link_edit.rs:227
link-edit-button-modify = Modify link

# Warning message about deleting links
# Source: ./ui/src/links/link_edit.rs:151
link-edit-delete-warning = Deleting a link is generally not recommended. Only delete links that haven't been published anywhere, or that you intentionally want to lead nowhere.

# Label for link code field
# Source: ./ui/src/links/link_edit.rs:96
link-edit-field-code = Code

# Label for description field
# Source: ./ui/src/links/link_edit.rs:52
link-edit-field-description = Description

# Label for QR code field
# Source: ./ui/src/links/link_edit.rs:116
link-edit-field-qrcode = QR Code

# Label for link target field
# Source: ./ui/src/links/link_edit.rs:76
link-edit-field-target = Redirect target

# Title for the link editing modal
# Source: ./ui/src/links/link_edit.rs:40
link-edit-modal-title = Edit a link

# Placeholder for code input
# Source: ./ui/src/links/link_edit.rs:102
link-edit-placeholder-code = Code

# Placeholder for description input
# Source: ./ui/src/links/link_edit.rs:62
link-edit-placeholder-description = Description

# Placeholder for target input
# Source: ./ui/src/links/link_edit.rs:82
link-edit-placeholder-target = Redirect target

# Button text to load more links
# Source: ./ui/src/links/mod.rs:385
links-button-load-more = Load more links

# Source: ./ui/src/links/link_display.rs:68
links-error-not-author = You can only edit links created by you.

# Text displayed while loading links data
# Source: ./ui/src/links/mod.rs:391
links-loading = Loading links

# Text for login link
# Source: ./ui/src/links/mod.rs:392
links-login = Login

# Placeholder text for filter input field
# Source: ./ui/src/links/mod.rs:272
links-table-filter-placeholder = Filter links by...

# Column header for link code
# Source: ./ui/src/links/mod.rs:233
links-table-header-code = Code

# Column header for description
# Source: ./ui/src/links/mod.rs:240
links-table-header-description = Description

# Source: ./ui/src/links/mod.rs:261
links-table-header-statistics = Statistics

# Column header for link target
# Source: ./ui/src/links/mod.rs:247
links-table-header-target = Short link target

# Column header for username
# Source: ./ui/src/links/mod.rs:254
links-table-header-username = Author

# The menu entry for login
# Source: ./ui/src/navbar.rs:111
login = Login

# The menu entry for logout
# Source: ./ui/src/navbar.rs:106
logout = Logout

# The title of the page
# Source: ./ui/src/navbar.rs:137
page-not-found = 404 Page not found

# The text of the page
# Source: ./ui/src/navbar.rs:138
page-not-found-text = The requested page was not found.

# Source: ./ui/src/login.rs:69
password = Password

# The requested route on the 404 page
# Source: ./ui/src/navbar.rs:139
# Parameters: $route
requested-route = The requested route was {$route}

# The menu entry for links
# Source: ./ui/src/navbar.rs:69
short_urls = Short URLs

# Displayed as a tooltip when there have been no clicks on this link in the last 12 months.
# Source: ./ui/src/links/stats.rs:26
# Parameters: $count
tooltip_no_clicks = This link was not clicked in the last 12 months. Before that is was clicked {$count} times.

# Text below the click statistic graph
# Source: ./ui/src/links/stats.rs:18
# Parameters: $count
total_clicks = Total clicks: {$count}

# Button text for confirming user deletion
# Source: ./ui/src/users/user_edit.rs:283
user-edit-button-confirm-delete = Actually delete user

# Button text for creating a new user
# Source: ./ui/src/users/user_edit.rs:226
user-edit-button-create = Create user

# Button text for deleting a user
# Source: ./ui/src/users/user_edit.rs:238
user-edit-button-delete = Delete user

# Button text for updating an existing user
# Source: ./ui/src/users/user_edit.rs:257
user-edit-button-update = Edit user

# Warning message displayed when attempting to delete a user
# Source: ./ui/src/users/user_edit.rs:178
user-edit-delete-warning = Deleting a user is usually not sensible as their created links would become ownerless. It's better to simply change the password.

# Label for email field in edit form
# Source: ./ui/src/users/user_edit.rs:70
user-edit-label-email = Email

# Label for password field in edit form
# Source: ./ui/src/users/user_edit.rs:90
user-edit-label-password = Password

# Label for role selection dropdown in edit form
# Source: ./ui/src/users/user_edit.rs:120
user-edit-label-role = Role

# Label for username field in edit form
# Source: ./ui/src/users/user_edit.rs:46
user-edit-label-username = Username

# Placeholder text for email input field
# Source: ./ui/src/users/user_edit.rs:76
user-edit-placeholder-email = Email

# Placeholder text for password input field
# Source: ./ui/src/users/user_edit.rs:96
user-edit-placeholder-password = Password

# Placeholder text for username input field
# Source: ./ui/src/users/user_edit.rs:56
user-edit-placeholder-username = Username

# Option for admin role in dropdown
# Source: ./ui/src/users/user_edit.rs:142
user-edit-role-admin = Administrator

# Option for disabled role in dropdown
# Source: ./ui/src/users/user_edit.rs:149
user-edit-role-disabled = Disabled

# Option for regular user role in dropdown
# Source: ./ui/src/users/user_edit.rs:137
user-edit-role-regular = Regular

# Title of the username in the edit dialog
# Source: ./ui/src/users/user_edit.rs:310
# Parameters: $username
user-edit-title = User data

# Source: ./ui/src/login.rs:45
username = Username

# The menu entry for users
# Source: ./ui/src/navbar.rs:72
users = User Accounts

# Button text to load more users
# Source: ./ui/src/users/mod.rs:313
users-button-load-more = Load more users...

# Text displayed while loading user data
# Source: ./ui/src/users/mod.rs:323
users-loading = Loading users...

# Text for login link
# Source: ./ui/src/users/mod.rs:324
users-login = Login

# Source: ./ui/src/users/user_display.rs:56
users-role-admin = Administrator

# Source: ./ui/src/users/user_display.rs:52
users-role-anonymous = Anonymous

# Source: ./ui/src/users/user_display.rs:54
users-role-disabled = Disabled

# Source: ./ui/src/users/user_display.rs:55
users-role-regular = Regular

# Placeholder text for filter input field
# Source: ./ui/src/users/mod.rs:233
users-table-filter-placeholder = Filter users by...

# Column header for email address
# Source: ./ui/src/users/mod.rs:217
users-table-header-email = Email

# Column header for user role
# Source: ./ui/src/users/mod.rs:219
users-table-header-role = Permission

# Column header for user ID
# Source: ./ui/src/users/mod.rs:203
users-table-header-user-id = User ID

# Column header for username
# Source: ./ui/src/users/mod.rs:210
users-table-header-username = Username

# Welcome message with the username
# Source: ./ui/src/home.rs:11
# Parameters: $username
welcome = Welcome {$username}

# Welcome message for strangers
# Source: ./ui/src/home.rs:13
welcome-stranger = Welcome stranger

# Source: ./ui/src/navbar.rs:93
# Parameters: $username
welcome-user = Welcome {$username}

