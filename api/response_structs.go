package api

type UserResp struct {
	Id                  int64   `json:"id"`
	Name                string  `json:"name"`
	DisplayName         *string `json:"displayName"`
	Country             string  `json:"country"`
	Bio                 *string `json:"bio"`
	HighlightedProjects []int64 `json:"highlightedProjects"`
	ProfilePicture      string  `json:"profilePicture"`
	JoinDate            string  `json:"joinDate"`
	BannerImage         *string `json:"bannerImage"`
	FollowerCount       int     `json:"followerCount"`
	FollowingCount      int     `json:"followingCount"`
	Verified            bool    `json:"verified"`
	Theme               *string `json:"theme"`
	ProjectCount        int64   `json:"projectCount"`
	HatchTeam           bool    `json:"hatchTeam"`
}

type Author struct {
	Id             int64   `json:"id"`
	Username       string  `json:"username"`
	ProfilePicture string  `json:"profilePicture"`
	DisplayName    *string `json:"displayName,omitempty"`
}

type ProjectResp struct {
	Id     int64  `json:"id"`
	Author Author `json:"author"`
	UploadTs int64 `json:"uploadTs"`
	Title string `json:"title"`
	Description string `json:"description"`
	Version *uint `json:"version,omitempty"`
	Rating string `json:"rating"`
	Thumbnail string `json:"thumbnail"`
	CommentCount uint32 `json:"commentCount"`
	Upvotes uint32 `json:"upvotes"`
	Downvotes uint32 `json:"downvotes"`
}

type ProjectsResp struct {
	Projects []ProjectResp `json:"projects"`
}
