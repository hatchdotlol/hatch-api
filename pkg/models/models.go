package models

type UserResp struct {
	Id                  int64   `json:"id"`
	Name                string  `json:"name"`
	DisplayName         *string `json:"displayName"`
	Country             string  `json:"country"`
	Bio                 *string `json:"bio"`
	HighlightedProjects []int64 `json:"highlightedProjects"`
	ProfilePicture      string  `json:"-"`
	JoinDate            string  `json:"joinDate"`
	BannerImage         *string `json:"bannerImage"`
	FollowerCount       int     `json:"followerCount"`
	FollowingCount      int     `json:"followingCount"`
	Verified            bool    `json:"verified"`
	Theme               *string `json:"theme"`
	ProjectCount        int64   `json:"projectCount"`
	HatchTeam           bool    `json:"hatchTeam"`
	Banned              *bool   `json:"banned,omitempty"`
}

type Author struct {
	Id          int64   `json:"id"`
	Username    string  `json:"username"`
	DisplayName *string `json:"displayName,omitempty"`
}

type ProjectResp struct {
	Id           int64  `json:"id"`
	Author       Author `json:"author"`
	UploadTs     int64  `json:"uploadTs"`
	Title        string `json:"title"`
	Description  string `json:"description"`
	Version      *uint  `json:"version,omitempty"`
	Rating       string `json:"rating"`
	Thumbnail    string `json:"-"`
	CommentCount int64  `json:"commentCount"`
	Upvotes      int64  `json:"upvotes"`
	Downvotes    int64  `json:"downvotes"`
}
